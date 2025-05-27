use super::ResultCode;
use crate::command_spec::CommandSpec;
use crate::config::HTTPProxyConfig;
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
pub struct CondaEnvironmentFromManifest {
    pub micromamba_binary_path: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub root_prefix: Utf8PathBuf,
    pub prefix: Utf8PathBuf,
    pub http_proxy_config: HTTPProxyConfig,
    pub build_timeout: u64,
    pub build_runtime_directory: Utf8PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CondaEnvironmentFromArchive {
    pub micromamba_binary_path: Utf8PathBuf,
    pub archive_path: Utf8PathBuf,
    pub root_prefix: Utf8PathBuf,
    pub prefix: Utf8PathBuf,
    pub build_timeout: u64,
    pub build_runtime_directory: Utf8PathBuf,
}

impl CondaEnvironmentFromManifest {
    pub fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        match session.run(&RunSpec {
            id: &format!("robotmk_env_create_{id}"),
            command_spec: &self.create_build_command_spec(),
            runtime_base_path: &self.build_runtime_directory.join("create"),
            timeout: self.build_timeout,
            cancellation_token,
        }) {
            Ok(Outcome::Completed(0)) => {
                info!("Environment building succeeded for plan {id}");
                let duration = (Utc::now() - start_time).num_seconds();
                Ok(BuildOutcome::Success(duration))
            }
            Ok(Outcome::Completed(_exit_code)) => {
                error!("Environment building not successful, plan {id} will be dropped");
                Ok(BuildOutcome::Error(format!(
                    "Environment building not successful, see {} for stdio logs",
                    self.build_runtime_directory
                )))
            }
            Ok(Outcome::Timeout) => {
                error!("Environment building timed out, plan {id} will be dropped");
                Ok(BuildOutcome::Timeout)
            }
            Ok(Outcome::Cancel) => {
                error!("Environment building cancelled");
                Err(Cancelled {})
            }
            Err(e) => {
                let error_with_context = e.context(anyhow!(
                    "Environment building failed, see {} for stdio logs",
                    self.build_runtime_directory
                ));
                let build_outcome = BuildOutcome::Error(format!("{error_with_context:?}"));
                error!(
                    "{:?}",
                    error_with_context.context(format!("Plan {id} will be dropped"))
                );
                Ok(build_outcome)
            }
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        wrap_into_conda_environment(
            &self.micromamba_binary_path,
            &self.root_prefix,
            &self.prefix,
            command_spec,
        )
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            _ => ResultCode::Error(format!("Failure with exit code {exit_code}")),
        }
    }

    fn create_build_command_spec(&self) -> CommandSpec {
        let mut build_command_spec = CommandSpec::new(&self.micromamba_binary_path);
        build_command_spec
            .add_argument("create")
            .add_argument("--file")
            .add_argument(&self.manifest_path)
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
}

impl CondaEnvironmentFromArchive {
    pub fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        info!("Extracting archive \"{}\"", self.archive_path);

        if let Err(error) = self.unpack() {
            error!("Archive unpacking failed: {error:?}");
            return Ok(BuildOutcome::Error(format!(
                "Archive unpacking failed: {error:?}"
            )));
        }

        let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
        if elapsed >= self.build_timeout {
            error!("Environment import timed out, plan {id} will be dropped");
            return Ok(BuildOutcome::Timeout);
        };

        match session.run(&RunSpec {
            id: &format!("robotmk_env_conda-unpack_{id}"),
            command_spec: &self.wrap(CommandSpec::new("conda-unpack")),
            runtime_base_path: &self.build_runtime_directory.join("conda-unpack"),
            timeout: self.build_timeout - elapsed,
            cancellation_token,
        }) {
            Ok(Outcome::Completed(0)) => {
                info!("Environment unpacking succeeded for plan {id}");
                let duration = (Utc::now() - start_time).num_seconds();
                Ok(BuildOutcome::Success(duration))
            }
            Ok(Outcome::Completed(_exit_code)) => {
                error!("conda-unpack not successful, plan {id} will be dropped");
                Ok(BuildOutcome::Error(format!(
                    "conda-unpack not successful, see {} for stdio logs",
                    self.build_runtime_directory
                )))
            }
            Ok(Outcome::Timeout) => {
                error!("conda-unpack timed out, plan {id} will be dropped");
                Ok(BuildOutcome::Timeout)
            }
            Ok(Outcome::Cancel) => {
                error!("conda-unpack cancelled");
                Err(Cancelled {})
            }
            Err(e) => {
                let error_with_context = e.context(anyhow!(
                    "conda-unpack failed, see {} for stdio logs",
                    self.build_runtime_directory
                ));
                let build_outcome = BuildOutcome::Error(format!("{error_with_context:?}"));
                error!(
                    "{:?}",
                    error_with_context.context(format!("Plan {id} will be dropped"))
                );
                Ok(build_outcome)
            }
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        wrap_into_conda_environment(
            &self.micromamba_binary_path,
            &self.root_prefix,
            &self.prefix,
            command_spec,
        )
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            _ => ResultCode::Error(format!("Failure with exit code {exit_code}")),
        }
    }

    fn unpack(&self) -> anyhow::Result<()> {
        Archive::new(GzDecoder::new(File::open(&self.archive_path)?)).unpack(&self.prefix)?;
        Ok(())
    }
}

fn wrap_into_conda_environment(
    binary_path: &Utf8Path,
    root_prefix: &Utf8Path,
    prefix: &Utf8Path,
    command_spec: CommandSpec,
) -> CommandSpec {
    let mut wrapped_spec = CommandSpec::new(binary_path);
    wrapped_spec
        .add_argument("run")
        .add_argument("--root-prefix")
        .add_argument(root_prefix)
        .add_argument("--prefix")
        .add_argument(prefix)
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
            CondaEnvironmentFromManifest {
                micromamba_binary_path: "/micromamba".into(),
                manifest_path: "/env.yaml".into(),
                root_prefix: "/root".into(),
                prefix: "/env".into(),
                http_proxy_config: HTTPProxyConfig::default(),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
            }
            .wrap(to_be_wrapped.clone()),
            expected
        );
        assert_eq!(
            CondaEnvironmentFromArchive {
                micromamba_binary_path: "/micromamba".into(),
                archive_path: "/env.tar.gz".into(),
                root_prefix: "/root".into(),
                prefix: "/env".into(),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
            }
            .wrap(to_be_wrapped),
            expected
        );
    }

    #[test]
    fn conda_from_manifest_build_command_spec() {
        let build_command_spec = CondaEnvironmentFromManifest {
            micromamba_binary_path: "/micromamba".into(),
            manifest_path: "/env.yaml".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig::default(),
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        }
        .create_build_command_spec();
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
    fn conda_from_manifest_build_command_spec_with_proxies() {
        let build_command_spec = CondaEnvironmentFromManifest {
            micromamba_binary_path: "/micromamba".into(),
            manifest_path: "/env.yaml".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig {
                http: Some("http://user:pass@corp.com:8080".into()),
                https: Some("http://user:pass@corp.com:8080".into()),
            },
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        }
        .create_build_command_spec();
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
