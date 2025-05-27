use super::ResultCode;
use crate::command_spec::CommandSpec;
use crate::config::RCCEnvironmentConfig;
use crate::results::BuildOutcome;
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use log::{error, info};
use tokio_util::sync::CancellationToken;

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

impl RCCEnvironment {
    pub fn new(
        base_dir: &Utf8Path,
        robocorp_home: &Utf8Path,
        plan_id: &str,
        rcc_binary_path: &Utf8Path,
        config: &RCCEnvironmentConfig,
        build_runtime_directory: &Utf8Path,
    ) -> Self {
        Self {
            binary_path: rcc_binary_path.into(),
            remote_origin: config.remote_origin.clone(),
            catalog_zip: config.catalog_zip.clone(),
            robot_yaml_path: base_dir.join(&config.robot_yaml_path),
            controller: "robotmk".into(),
            space: plan_id.into(),
            build_timeout: config.build_timeout,
            build_runtime_directory: build_runtime_directory.into(),
            robocorp_home: robocorp_home.to_string(),
        }
    }

    pub fn bundled_command_spec(binary_path: &Utf8Path, robocorp_home: String) -> CommandSpec {
        let mut command_spec = CommandSpec::new(binary_path);
        command_spec.add_argument("--bundled");
        command_spec.add_plain_env("ROBOCORP_HOME", &robocorp_home);
        command_spec
    }

    pub fn build(
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

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
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

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::Success,
            10 => ResultCode::WrappedCommandFailed,
            _ => ResultCode::EnvironmentFailed,
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
                binary_path: "C:\\bin\\z.exe".into(),
                remote_origin: None,
                catalog_zip: None,
                robot_yaml_path: "C:\\some_synthetic_test\\robot.yaml".into(),
                controller: "robotmk".into(),
                space: "my_plan".into(),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
                robocorp_home: "~/.robocorp/".into(),
            }
            .wrap(to_be_wrapped),
            expected
        );
    }
}
