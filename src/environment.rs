use crate::command_spec::CommandSpec;
use crate::config::EnvironmentConfig;
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

pub enum ResultCode {
    Success,
    WrappedCommandFailed,
    EnvironmentFailed,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Environment {
    System(SystemEnvironment),
    Rcc(RCCEnvironment),
    CondaFromManifest(CondaEnvironmentFromManifest),
    CondaFromArchive(CondaEnvironmentFromArchive),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemEnvironment {}

#[derive(Clone, Debug, PartialEq)]
pub struct RCCEnvironment {
    pub binary_path: Utf8PathBuf,
    pub remote_origin: Option<String>,
    pub catalog_zip: Option<Utf8PathBuf>,
    pub robot_yaml_path: Utf8PathBuf,
    pub controller: String,
    pub space: String,
    pub build_timeout: u64,
    pub build_runtime_directory: Utf8PathBuf,
    pub robocorp_home: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CondaEnvironmentFromManifest {
    pub micromamba_binary_path: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub root_prefix: Utf8PathBuf,
    pub prefix: Utf8PathBuf,
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

impl Environment {
    pub fn new(
        base_dir: &Utf8Path,
        robocorp_home: &Utf8Path,
        plan_id: &str,
        rcc_binary_path: &Utf8Path,
        environment_config: &EnvironmentConfig,
        build_runtime_directory: &Utf8Path,
    ) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_binary_path.to_path_buf(),
                remote_origin: rcc_environment_config.remote_origin.clone(),
                catalog_zip: rcc_environment_config.catalog_zip.clone(),
                robot_yaml_path: base_dir.join(&rcc_environment_config.robot_yaml_path),
                controller: String::from("robotmk"),
                space: plan_id.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
                build_runtime_directory: build_runtime_directory.to_path_buf(),
                robocorp_home: robocorp_home.to_string(),
            }),
            EnvironmentConfig::Conda(_) => {
                panic!("Conda environments are not supported yet.")
            }
        }
    }

    pub fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        match self {
            Self::System(system_environment) => system_environment.build(),
            Self::Rcc(rcc_environment) => {
                rcc_environment.build(id, session, start_time, cancellation_token)
            }
            Self::CondaFromManifest(conda_environment_from_manifest) => {
                conda_environment_from_manifest.build(id, session, start_time, cancellation_token)
            }
            Self::CondaFromArchive(conda_environment_from_archive) => {
                conda_environment_from_archive.build(id, session, start_time, cancellation_token)
            }
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        match self {
            Self::System(system_environment) => system_environment.wrap(command_spec),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command_spec),
            Self::CondaFromManifest(conda_environment_from_manifest) => {
                conda_environment_from_manifest.wrap(command_spec)
            }
            Self::CondaFromArchive(conda_environment_from_archive) => {
                conda_environment_from_archive.wrap(command_spec)
            }
        }
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match self {
            Self::System(system_env) => system_env.create_result_code(exit_code),
            Self::Rcc(rcc_env) => rcc_env.create_result_code(exit_code),
            Self::CondaFromManifest(conda_env_from_manifest) => {
                conda_env_from_manifest.create_result_code(exit_code)
            }
            Self::CondaFromArchive(conda_env_from_archive) => {
                conda_env_from_archive.create_result_code(exit_code)
            }
        }
    }
}

impl SystemEnvironment {
    fn build(&self) -> Result<BuildOutcome, Cancelled> {
        Ok(BuildOutcome::NotNeeded)
    }

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        command_spec
    }

    fn create_result_code(&self, exit_code: i32) -> ResultCode {
        if exit_code == 0 {
            return ResultCode::Success;
        }
        ResultCode::WrappedCommandFailed
    }
}

impl RCCEnvironment {
    pub fn bundled_command_spec(binary_path: &Utf8Path, robocorp_home: String) -> CommandSpec {
        let mut command_spec = CommandSpec::new(binary_path);
        command_spec.add_argument("--bundled");
        command_spec.add_plain_env("ROBOCORP_HOME", &robocorp_home);
        command_spec
    }

    fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        if let Some(catalog_ip) = &self.catalog_zip {
            match self.run_catalog_import(
                id,
                catalog_ip,
                session,
                self.build_timeout,
                cancellation_token,
            ) {
                Ok(Outcome::Completed(0)) => {
                    info!("Environment import succeeded for plan {id}");
                }
                Ok(Outcome::Completed(_exit_code)) => {
                    error!("Environment import not successful, plan {id} will be dropped");
                    return Ok(BuildOutcome::Error(format!(
                        "Environment import not successful, see {} for stdio logs",
                        self.build_runtime_directory
                    )));
                }
                Ok(Outcome::Timeout) => {
                    error!("Environment import timed out, plan {id} will be dropped");
                    return Ok(BuildOutcome::Timeout);
                }
                Ok(Outcome::Cancel) => {
                    error!("Environment import cancelled");
                    return Err(Cancelled {});
                }
                Err(e) => {
                    let log_error = e.context(anyhow!(
                        "Environment import failed, plan {id} will be dropped. See {} for stdio logs",
                        self.build_runtime_directory
                    ));
                    error!("{log_error:?}");
                    return Ok(BuildOutcome::Error(format!("{log_error:?}")));
                }
            }
        }

        let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
        if elapsed >= self.build_timeout {
            error!("Environment import timed out, plan {id} will be dropped");
            return Ok(BuildOutcome::Timeout);
        };

        match self.run_noop_command(
            id,
            session,
            self.build_timeout - elapsed,
            cancellation_token,
        ) {
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
                let log_error = e.context(anyhow!(
                    "Environment building failed, plan {id} will be dropped. See {} for stdio logs",
                    self.build_runtime_directory
                ));
                error!("{log_error:?}");
                Ok(BuildOutcome::Error(format!("{log_error:?}")))
            }
        }
    }

    fn run_catalog_import(
        &self,
        id: &str,
        catalog_zip: &Utf8Path,
        session: &Session,
        timeout: u64,
        cancellation_token: &CancellationToken,
    ) -> anyhow::Result<Outcome<i32>> {
        let mut import_command_spec =
            Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
        import_command_spec
            .add_argument("holotree")
            .add_argument("import")
            .add_argument(catalog_zip);
        session.run(&RunSpec {
            id: &format!("robotmk_env_import_{id}"),
            command_spec: &import_command_spec,
            runtime_base_path: &self.build_runtime_directory.join("import"),
            timeout,
            cancellation_token,
        })
    }

    fn run_noop_command(
        &self,
        id: &str,
        session: &Session,
        timeout: u64,
        cancellation_token: &CancellationToken,
    ) -> anyhow::Result<Outcome<i32>> {
        let mut noop_command_spec =
            Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
        noop_command_spec
            .add_argument("task")
            .add_argument("script");
        self.apply_current_settings(&mut noop_command_spec);
        if let Some(remote_origin) = &self.remote_origin {
            noop_command_spec.add_obfuscated_env("RCC_REMOTE_ORIGIN", remote_origin);
        }
        noop_command_spec.add_argument("--").add_argument(
            #[cfg(unix)]
            "true",
            #[cfg(windows)]
            "cmd.exe",
        );

        session.run(&RunSpec {
            id: &format!("robotmk_env_building_{id}"),
            command_spec: &noop_command_spec,
            runtime_base_path: &self.build_runtime_directory.join("build"),
            timeout,
            cancellation_token,
        })
    }

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec =
            Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
        wrapped_spec
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build");
        self.apply_current_settings(&mut wrapped_spec);
        wrapped_spec
            .add_argument("--")
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

    fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            10 => ResultCode::WrappedCommandFailed,
            _ => ResultCode::EnvironmentFailed,
        }
    }

    fn apply_current_settings(&self, command_spec: &mut CommandSpec) {
        command_spec
            .add_argument("--robot")
            .add_argument(&self.robot_yaml_path)
            // According to the `rcc --help`:
            // --controller string   internal, DO NOT USE (unless you know what you are doing)
            //
            // According to https://github.com/robocorp/rcc/blob/v16.5.0/docs/recipes.md#how-to-control-holotree-environments
            // This is one of three ways to controller where holotree spaces are created [...] when
            // applications are calling rcc, they should have their own "controller" identity, so
            // that all spaces created for one application are groupped together by prefix of their
            // "space" identity name.
            //
            // According to https://github.com/robocorp/rcc/blob/v16.5.0/docs/vocabulary.md#controller
            // This is tool or context that is currently running rcc command.
            //
            // From the code we can see, that the controlle is included in UserAgent of HTTP
            // requests for and the journaling (for example).
            //
            // In sum, ignoring the `DO NOT USE` seems correct.
            .add_argument("--controller")
            .add_argument(&self.controller)
            .add_argument("--space")
            .add_argument(&self.space);
    }
}

impl CondaEnvironmentFromManifest {
    fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
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
        match session.run(&RunSpec {
            id: &format!("robotmk_env_create_{id}"),
            command_spec: &build_command_spec,
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

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        wrap_into_conda_environment(
            &self.micromamba_binary_path,
            &self.root_prefix,
            &self.prefix,
            command_spec,
        )
    }

    fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            _ => ResultCode::Error(format!("Failure with exit code {exit_code}")),
        }
    }
}

impl CondaEnvironmentFromArchive {
    fn build(
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

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        wrap_into_conda_environment(
            &self.micromamba_binary_path,
            &self.root_prefix,
            &self.prefix,
            command_spec,
        )
    }

    fn unpack(&self) -> anyhow::Result<()> {
        Archive::new(GzDecoder::new(File::open(&self.archive_path)?)).unpack(&self.prefix)?;
        Ok(())
    }

    fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            _ => ResultCode::Error(format!("Failure with exit code {exit_code}")),
        }
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

    fn command_spec_for_wrap() -> CommandSpec {
        let mut command_spec = CommandSpec::new("C:\\x\\y\\z.exe");
        command_spec
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        command_spec
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");
        command_spec
    }

    #[test]
    fn system_wrap() {
        assert_eq!(
            SystemEnvironment {}.wrap(command_spec_for_wrap()),
            command_spec_for_wrap()
        );
    }

    #[test]
    fn rcc_wrap() {
        let mut expected = CommandSpec::new("C:\\bin\\z.exe");
        expected
            .add_argument("--bundled")
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build")
            .add_argument("--robot")
            .add_argument("C:\\some_synthetic_test\\robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_plan")
            .add_argument("--")
            .add_argument("C:\\x\\y\\z.exe")
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value")
            .add_plain_env("ROBOCORP_HOME", "~/.robocorp/")
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");
        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                remote_origin: None,
                catalog_zip: None,
                robot_yaml_path: Utf8PathBuf::from("C:\\some_synthetic_test\\robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_plan"),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
                robocorp_home: String::from("~/.robocorp/"),
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }

    #[test]
    fn conda_wrap() {
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
                micromamba_binary_path: Utf8PathBuf::from("/micromamba"),
                manifest_path: Utf8PathBuf::from("/env.yaml"),
                root_prefix: Utf8PathBuf::from("/root"),
                prefix: Utf8PathBuf::from("/env"),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
        assert_eq!(
            CondaEnvironmentFromArchive {
                micromamba_binary_path: Utf8PathBuf::from("/micromamba"),
                archive_path: Utf8PathBuf::from("/env.tar.gz"),
                root_prefix: Utf8PathBuf::from("/root"),
                prefix: Utf8PathBuf::from("/env"),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }
}
