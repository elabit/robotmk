use super::ResultCode;
use super::robotmk_env_manifest::parse_robotmk_environment_manifest;
use crate::command_spec::CommandSpec;
use crate::config::{CondaEnvironmentSource, HTTPProxyConfig, TlsCertificateValidation};
use crate::results::BuildOutcome;
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use log::{error, info};
use std::fs::File;
use std::vec;
use tar::Archive;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub struct CondaEnvironment {
    pub source: CondaEnvironmentSource,
    pub robotmk_manifest_path: Option<Utf8PathBuf>,
    pub micromamba_binary_path: Utf8PathBuf,
    pub root_prefix: Utf8PathBuf,
    pub prefix: Utf8PathBuf,
    pub http_proxy_config: HTTPProxyConfig,
    pub tls_certificate_validation: TlsCertificateValidation,
    pub tls_revokation_enabled: bool,
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
        if let BuildStepOutcome::Failure(failure) = match self.source {
            CondaEnvironmentSource::Manifest(ref manifest_path) => {
                self.build_from_manifest(manifest_path, id, session, cancellation_token)
            }
            CondaEnvironmentSource::Archive(ref archive_path) => {
                self.build_from_archive(archive_path, id, session, start_time, cancellation_token)
            }
        } {
            return failure.into();
        }

        if let BuildStepOutcome::Failure(failure) = self.run_post_build_commands(
            &match self.gather_post_build_commands() {
                Ok(commands) => commands,
                Err(e) => {
                    let build_outcome = BuildOutcome::Error(format!("{e:?}"));
                    error!("{:?}", e.context(format!("Plan {id} will be dropped")));
                    return Ok(build_outcome);
                }
            },
            id,
            session,
            start_time,
            cancellation_token,
        ) {
            return failure.into();
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

    fn run_build_step(
        run_spec: &RunSpec,
        session: &Session,
        build_step_label: &str,
        plan_id: &str,
    ) -> BuildStepOutcome {
        match session.run(run_spec) {
            Ok(Outcome::Completed(0)) => {
                info!("Plan {plan_id}: {build_step_label}: success");
                BuildStepOutcome::Success
            }
            Ok(Outcome::Completed(_exit_code)) => {
                error!(
                    "Plan {plan_id}: {build_step_label}: non-zero exit code, plan will be dropped"
                );
                BuildStepOutcome::Failure(BuildStepOutcomeFailure::Error(format!(
                    "{build_step_label}: non-zero exit code, see {stdio_location} for stdio logs",
                    stdio_location = run_spec.runtime_base_path
                )))
            }
            Ok(Outcome::Timeout) => {
                error!("Plan {plan_id}: {build_step_label}: timeout, plan will be dropped");
                BuildStepOutcome::Failure(BuildStepOutcomeFailure::Timeout)
            }
            Ok(Outcome::Cancel) => {
                error!("Plan {plan_id}: {build_step_label}: cancelled");
                BuildStepOutcome::Failure(BuildStepOutcomeFailure::Cancelled)
            }
            Err(e) => {
                let error_with_context = e.context(anyhow!(
                    "{build_step_label}: failure, see {stdio_location} for stdio logs",
                    stdio_location = run_spec.runtime_base_path
                ));
                let failure = BuildStepOutcomeFailure::Error(format!("{error_with_context:?}"));
                error!(
                    "{:?}",
                    error_with_context.context(format!("Plan {plan_id} will be dropped"))
                );
                BuildStepOutcome::Failure(failure)
            }
        }
    }

    fn build_from_manifest(
        &self,
        manifest_path: &Utf8Path,
        id: &str,
        session: &Session,
        cancellation_token: &CancellationToken,
    ) -> BuildStepOutcome {
        info!("Building Conda environment from manifest for plan {id}");
        Self::run_build_step(
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
        match &self.tls_certificate_validation {
            TlsCertificateValidation::Enabled => {}
            TlsCertificateValidation::Disabled => {
                build_command_spec
                    .add_argument("--ssl-verify")
                    .add_argument("false");
            }
            TlsCertificateValidation::EnabledWithCustomCert(cert_path) => {
                build_command_spec
                    .add_argument("--ssl-verify")
                    .add_argument(cert_path);
            }
        }
        if !self.tls_revokation_enabled {
            build_command_spec.add_argument("--ssl-no-revoke");
        }
        if !self.http_proxy_config.no_proxy.is_empty() {
            build_command_spec
                .add_obfuscated_env("NO_PROXY", &self.http_proxy_config.no_proxy.join(","));
        }
        if let Some(http_proxy) = &self.http_proxy_config.http {
            build_command_spec.add_obfuscated_env("HTTP_PROXY", http_proxy);
        }
        if let Some(https_proxy) = &self.http_proxy_config.https {
            build_command_spec.add_obfuscated_env("HTTPS_PROXY", https_proxy);
        }
        build_command_spec
    }

    fn build_from_archive(
        &self,
        archive_path: &Utf8Path,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> BuildStepOutcome {
        info!("Extracting archive {archive_path} for plan {id}");

        if let Err(error) = self.unpack(archive_path) {
            error!("Archive unpacking failed: {error:?}");
            return BuildStepOutcome::Failure(BuildStepOutcomeFailure::Error(format!(
                "Archive unpacking failed: {error:?}"
            )));
        }

        let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
        if elapsed >= self.build_timeout {
            error!("Environment import timed out, plan {id} will be dropped");
            return BuildStepOutcome::Failure(BuildStepOutcomeFailure::Timeout);
        };

        info!("Running conda-unpack for plan {id}");
        Self::run_build_step(
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

    fn gather_post_build_commands(&self) -> anyhow::Result<Vec<(String, CommandSpec)>> {
        let robotmk_manifest_path = if let Some(path) = &self.robotmk_manifest_path {
            path
        } else {
            return Ok(vec![]);
        };

        let manifest = parse_robotmk_environment_manifest(robotmk_manifest_path)?;
        let mut post_build_commands = vec![];

        for post_build_command in manifest.post_build_commands {
            let command_spec: Option<CommandSpec> = (&post_build_command).into();
            match command_spec {
                None => {
                    info!(
                        "Post-build command `{}` is empty, skipping",
                        post_build_command.name
                    );
                    continue;
                }
                Some(command_spec) => post_build_commands.push((
                    post_build_command.name,
                    self.wrap_post_build_command_spec(command_spec),
                )),
            };
        }

        Ok(post_build_commands)
    }

    fn wrap_post_build_command_spec(&self, mut post_build_command: CommandSpec) -> CommandSpec {
        // Setting HTTP(S)_PROXY is of course no universal way to configure proxies for any command.
        // However, Playwright respects these environment variables:
        // https://playwright.dev/docs/browsers#install-behind-a-firewall-or-a-proxy
        // This is particularly important in the context of the `rfbrowser init` command.
        if let Some(http_proxy) = &self.http_proxy_config.http {
            post_build_command.add_obfuscated_env("HTTP_PROXY", http_proxy);
        }
        if let Some(https_proxy) = &self.http_proxy_config.https {
            post_build_command.add_obfuscated_env("HTTPS_PROXY", https_proxy);
        }
        self.wrap(post_build_command)
    }

    fn run_post_build_commands(
        &self,
        post_build_command_specs: &[(String, CommandSpec)],
        plan_id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> BuildStepOutcome {
        if post_build_command_specs.is_empty() {
            info!("No post-build commands found for plan {plan_id}, skipping");
            return BuildStepOutcome::Success;
        }

        for (command_name, command_spec) in post_build_command_specs {
            let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
            if elapsed >= self.build_timeout {
                error!("Timeout while running post-build commands, plan {plan_id} will be dropped");
                return BuildStepOutcome::Failure(BuildStepOutcomeFailure::Timeout);
            };
            info!("Running post-build command {command_name} for plan {plan_id}");
            if let BuildStepOutcome::Failure(failure) = Self::run_build_step(
                &RunSpec {
                    id: &format!("robotmk_env_{plan_id}_post_build_{command_name}"),
                    command_spec,
                    runtime_base_path: &self
                        .build_runtime_directory
                        .join(format!("post_build_{command_name}")),
                    timeout: self.build_timeout - elapsed,
                    cancellation_token,
                },
                session,
                &format!("Post-build command {command_name}"),
                plan_id,
            ) {
                return BuildStepOutcome::Failure(failure);
            }
        }

        info!("Post-build commands for plan {plan_id} completed successfully");
        BuildStepOutcome::Success
    }
}

enum BuildStepOutcome {
    Success,
    Failure(BuildStepOutcomeFailure),
}

enum BuildStepOutcomeFailure {
    Timeout,
    Error(String),
    Cancelled,
}

impl From<BuildStepOutcomeFailure> for Result<BuildOutcome, Cancelled> {
    fn from(failure: BuildStepOutcomeFailure) -> Self {
        match failure {
            BuildStepOutcomeFailure::Timeout => Ok(BuildOutcome::Timeout),
            BuildStepOutcomeFailure::Error(error) => Ok(BuildOutcome::Error(error)),
            BuildStepOutcomeFailure::Cancelled => Err(Cancelled {}),
        }
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
                robotmk_manifest_path: None,
                micromamba_binary_path: "/micromamba".into(),
                root_prefix: "/root".into(),
                prefix: "/env".into(),
                http_proxy_config: HTTPProxyConfig::default(),
                tls_certificate_validation: TlsCertificateValidation::Enabled,
                tls_revokation_enabled: false,
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
            robotmk_manifest_path: None,
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig::default(),
            tls_certificate_validation: TlsCertificateValidation::Disabled,
            tls_revokation_enabled: false,
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
                "/env",
                "--ssl-verify",
                "false",
                "--ssl-no-revoke"
            ]
        );
        assert!(build_command_spec.envs_rendered_plain.is_empty());
        assert!(build_command_spec.envs_rendered_obfuscated.is_empty());
    }

    #[test]
    fn make_create_command_spec_with_proxies() {
        let build_command_spec = CondaEnvironment {
            source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
            robotmk_manifest_path: None,
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig {
                no_proxy: vec!["localhost".into()],
                http: Some("http://user:pass@corp.com:8080".into()),
                https: Some("http://user:pass@corp.com:8080".into()),
            },
            tls_certificate_validation: TlsCertificateValidation::Enabled,
            tls_revokation_enabled: false,
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
                "/env",
                "--ssl-no-revoke"
            ]
        );
        assert!(build_command_spec.envs_rendered_plain.is_empty());
        assert_eq!(
            build_command_spec.envs_rendered_obfuscated,
            [
                ("NO_PROXY".into(), "localhost".into()),
                ("HTTP_PROXY".into(), "http://user:pass@corp.com:8080".into()),
                (
                    "HTTPS_PROXY".into(),
                    "http://user:pass@corp.com:8080".into()
                ),
            ]
        );
    }

    #[test]
    fn wrap_post_build_command_spec() {
        let mut to_be_wrapped = CommandSpec::new("rfbrowser");
        to_be_wrapped.add_argument("init");

        let env = CondaEnvironment {
            source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
            robotmk_manifest_path: None,
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig::default(),
            tls_certificate_validation: TlsCertificateValidation::Enabled,
            tls_revokation_enabled: false,
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        };

        assert_eq!(
            env.wrap_post_build_command_spec(to_be_wrapped.clone()),
            env.wrap(to_be_wrapped)
        );
    }

    #[test]
    fn wrap_post_build_command_spec_with_proxies() {
        let mut to_be_wrapped = CommandSpec::new("rfbrowser");
        to_be_wrapped.add_argument("init");

        let env = CondaEnvironment {
            source: CondaEnvironmentSource::Manifest("/env.yaml".into()),
            robotmk_manifest_path: None,
            micromamba_binary_path: "/micromamba".into(),
            root_prefix: "/root".into(),
            prefix: "/env".into(),
            http_proxy_config: HTTPProxyConfig {
                no_proxy: vec![],
                http: Some("http://user:pass@corp.com:8080".into()),
                https: Some("http://user:pass@corp.com:8080".into()),
            },
            tls_certificate_validation: TlsCertificateValidation::Enabled,
            tls_revokation_enabled: false,
            build_timeout: 600,
            build_runtime_directory: Utf8PathBuf::default(),
        };

        let mut expected = env.wrap(to_be_wrapped.clone());
        expected.add_obfuscated_env("HTTP_PROXY", "http://user:pass@corp.com:8080");
        expected.add_obfuscated_env("HTTPS_PROXY", "http://user:pass@corp.com:8080");

        assert_eq!(env.wrap_post_build_command_spec(to_be_wrapped), expected);
    }
}
