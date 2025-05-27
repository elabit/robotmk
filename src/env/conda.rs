use super::ResultCode;
use crate::command_spec::CommandSpec;
use crate::config::{CondaEnvironmentSource, HTTPProxyConfig};
use crate::results::BuildOutcome;
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use log::{error, info};
use std::fs::File;
use tar::Archive;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub struct CondaEnvironment {
    pub source: CondaEnvironmentSource,
    pub micromamba_binary_path: Utf8PathBuf,
    pub root_prefix: Utf8PathBuf,
    pub prefix: Utf8PathBuf,
    pub http_proxy_config: HTTPProxyConfig,
    pub build_timeout: u64,
    pub build_runtime_directory: Utf8PathBuf,
}

impl CondaEnvironment {
    pub fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        if let Some(build_result) = match self.source {
            CondaEnvironmentSource::Manifest(ref manifest_path) => self
                .build_from_manifest_and_report_if_unsuccessful(
                    manifest_path,
                    id,
                    session,
                    cancellation_token,
                ),
            CondaEnvironmentSource::Archive(ref archive_path) => self
                .build_from_archive_and_report_if_unsuccessful(
                    archive_path,
                    id,
                    session,
                    start_time,
                    cancellation_token,
                ),
        } {
            return build_result;
        }

        Ok(BuildOutcome::Success(
            (Utc::now() - start_time).num_seconds(),
        ))
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec = CommandSpec::new(&self.micromamba_binary_path);
        wrapped_spec
            .add_argument("run")
            .add_argument("--root-prefix")
            .add_argument(&self.root_prefix)
            .add_argument("--prefix")
            .add_argument(&self.prefix)
            .add_argument(command_spec.executable)
            .add_arguments(command_spec.arguments);
        for (key, value) in command_spec.envs_rendered_plain {
            wrapped_spec.add_plain_env(key, value);
        }
        for (key, value) in command_spec.envs_rendered_obfuscated {
            wrapped_spec.add_obfuscated_env(key, value);
        }
        wrapped_spec
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            _ => ResultCode::Error(format!("Failure with exit code {exit_code}")),
        }
    }

    fn run_build_step_and_report_if_unsuccessful(
        run_spec: &RunSpec,
        session: &Session,
        build_step_label: &str,
        plan_id: &str,
    ) -> Option<Result<BuildOutcome, Cancelled>> {
        match session.run(run_spec) {
            Ok(Outcome::Completed(0)) => {
                info!("Plan {plan_id}: {build_step_label}: success");
                None
            }
            Ok(Outcome::Completed(_exit_code)) => {
                error!(
                    "Plan {plan_id}: {build_step_label}: non-zero exit code, plan will be dropped"
                );
                Some(Ok(BuildOutcome::Error(format!(
                    "{build_step_label}: non-zero exit code, see {stdio_location} for stdio logs",
                    stdio_location = run_spec.runtime_base_path
                ))))
            }
            Ok(Outcome::Timeout) => {
                error!("Plan {plan_id}: {build_step_label}: timeout, plan will be dropped");
                Some(Ok(BuildOutcome::Timeout))
            }
            Ok(Outcome::Cancel) => {
                error!("Plan {plan_id}: {build_step_label}: cancelled");
                Some(Err(Cancelled {}))
            }
            Err(e) => {
                let error_with_context = e.context(anyhow!(
                    "{build_step_label}: failure, see {stdio_location} for stdio logs",
                    stdio_location = run_spec.runtime_base_path
                ));
                let build_outcome = BuildOutcome::Error(format!("{error_with_context:?}"));
                error!(
                    "{:?}",
                    error_with_context.context(format!("Plan {plan_id} will be dropped"))
                );
                Some(Ok(build_outcome))
            }
        }
    }

    fn build_from_manifest_and_report_if_unsuccessful(
        &self,
        manifest_path: &Utf8Path,
        id: &str,
        session: &Session,
        cancellation_token: &CancellationToken,
    ) -> Option<Result<BuildOutcome, Cancelled>> {
        info!("Building Conda environment from manifest for plan {id}");
        Self::run_build_step_and_report_if_unsuccessful(
            &RunSpec {
                id: &format!("robotmk_env_create_{id}"),
                command_spec: &self.make_create_command_spec(manifest_path),
                runtime_base_path: &self.build_runtime_directory.join("create"),
                timeout: self.build_timeout,
                cancellation_token,
            },
            session,
            "Environment creation",
            id,
        )
    }

    fn make_create_command_spec(&self, manifest_path: &Utf8Path) -> CommandSpec {
        let mut build_command_spec = CommandSpec::new(&self.micromamba_binary_path);
        build_command_spec
            .add_argument("create")
            .add_argument("--file")
            .add_argument(manifest_path)
            .add_argument("--yes")
            .add_argument("--root-prefix")
            .add_argument(&self.root_prefix)
            .add_argument("--prefix")
            .add_argument(&self.prefix);
        if let Some(http_proxy) = &self.http_proxy_config.http {
            build_command_spec.add_obfuscated_env("HTTP_PROXY", http_proxy);
        }
        if let Some(https_proxy) = &self.http_proxy_config.https {
            build_command_spec.add_obfuscated_env("HTTPS_PROXY", https_proxy);
        }
        build_command_spec
    }

    fn build_from_archive_and_report_if_unsuccessful(
        &self,
        archive_path: &Utf8Path,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Option<Result<BuildOutcome, Cancelled>> {
        info!("Extracting archive {archive_path} for plan {id}");

        if let Err(error) = self.unpack(archive_path) {
            error!("Archive unpacking failed: {error:?}");
            return Some(Ok(BuildOutcome::Error(format!(
                "Archive unpacking failed: {error:?}"
            ))));
        }

        let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
        if elapsed >= self.build_timeout {
            error!("Environment import timed out, plan {id} will be dropped");
            return Some(Ok(BuildOutcome::Timeout));
        };

        info!("Running conda-unpack for plan {id}");
        Self::run_build_step_and_report_if_unsuccessful(
            &RunSpec {
                id: &format!("robotmk_env_conda-unpack_{id}"),
                command_spec: &self.wrap(CommandSpec::new("conda-unpack")),
                runtime_base_path: &self.build_runtime_directory.join("conda-unpack"),
                timeout: self.build_timeout - elapsed,
                cancellation_token,
            },
            session,
            "conda-unpack",
            id,
        )
    }

    fn unpack(&self, archive_path: &Utf8Path) -> anyhow::Result<()> {
        Archive::new(GzDecoder::new(File::open(archive_path)?)).unpack(&self.prefix)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap() {
        let mut to_be_wrapped = CommandSpec::new("C:\\x\\y\\z.exe");
        to_be_wrapped
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        to_be_wrapped
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");

        let mut expected = CommandSpec::new("/micromamba");
        expected
            .add_argument("run")
            .add_argument("--root-prefix")
            .add_argument("/root")
            .add_argument("--prefix")
            .add_argument("/env")
            .add_argument("C:\\x\\y\\z.exe")
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value")
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");
        assert_eq!(
            CondaEnvironment {
                source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
                micromamba_binary_path: "/micromamba".into(),
                root_prefix: "/root".into(),
                prefix: "/env".into(),
                http_proxy_config: HTTPProxyConfig::default(),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
            }
            .wrap(to_be_wrapped.clone()),
            expected
        );
    }

    #[test]
    fn make_create_command_spec() {
        let build_command_spec = CondaEnvironment {
            source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig::default(),
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        }
        .make_create_command_spec("/env.yaml".into());
        assert_eq!(build_command_spec.executable, "/micromamba");
        assert_eq!(
            build_command_spec.arguments,
            [
                "create",
                "--file",
                "/env.yaml",
                "--yes",
                "--root-prefix",
                "/root",
                "--prefix",
                "/env"
            ]
        );
        assert!(build_command_spec.envs_rendered_plain.is_empty());
        assert!(build_command_spec.envs_rendered_obfuscated.is_empty());
    }

    #[test]
    fn make_create_command_spec_with_proxies() {
        let build_command_spec = CondaEnvironment {
            source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig {
                http: Some("http://user:pass@corp.com:8080".into()),
                https: Some("http://user:pass@corp.com:8080".into()),
            },
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        }
        .make_create_command_spec("/env.yaml".into());
        assert_eq!(build_command_spec.executable, "/micromamba");
        assert_eq!(
            build_command_spec.arguments,
            vec![
                "create",
                "--file",
                "/env.yaml",
                "--yes",
                "--root-prefix",
                "/root",
                "--prefix",
                "/env"
            ]
        );
        assert!(build_command_spec.envs_rendered_plain.is_empty());
        assert_eq!(
            build_command_spec.envs_rendered_obfuscated,
            [
                ("HTTP_PROXY".into(), "http://user:pass@corp.com:8080".into()),
                (
                    "HTTPS_PROXY".into(),
                    "http://user:pass@corp.com:8080".into()
                ),
            ]
        );
    }
}
