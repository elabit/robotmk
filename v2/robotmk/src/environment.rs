use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use super::command_spec::CommandSpec;
use super::config::external::{Config, EnvironmentConfig};
use super::results::{EnvironmentBuildStatesAdministrator, EnvironmentBuildStatus};
use super::termination::TerminationFlag;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error, info};

pub fn environment_building_stdio_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("environment_building_stdio")
}

pub fn build_environments(config: &Config, termination_flag: &TerminationFlag) -> Result<()> {
    let suites = config.suites();
    let mut environment_build_states_administrator =
        EnvironmentBuildStatesAdministrator::new_with_pending(
            suites
                .iter()
                .map(|(suite_name, _suite_config)| suite_name.to_owned()),
            &config.working_directory,
            &config.results_directory,
        );
    environment_build_states_administrator.write_atomic()?;
    let env_building_stdio_directory =
        environment_building_stdio_directory(&config.working_directory);

    for (suite_name, suite_config) in suites {
        match Environment::new(suite_name, &suite_config.environment_config).build_instructions() {
            Some(build_instructions) => {
                info!("Building environment for suite {}", suite_name);
                environment_build_states_administrator
                    .insert_and_write_atomic(suite_name, EnvironmentBuildStatus::InProgress)?;
                environment_build_states_administrator.insert_and_write_atomic(
                    suite_name,
                    run_environment_build(ChildProcessSupervisor {
                        command_spec: &build_instructions.command_spec,
                        stdio_paths: Some(StdioPaths {
                            stdout: env_building_stdio_directory
                                .join(format!("{suite_name}.stdout")),
                            stderr: env_building_stdio_directory
                                .join(format!("{suite_name}.stderr")),
                        }),
                        timeout: build_instructions.timeout,
                        termination_flag,
                    })?,
                )?;
            }
            None => {
                debug!("Nothing to do for suite {}", suite_name);
                environment_build_states_administrator
                    .insert_and_write_atomic(suite_name, EnvironmentBuildStatus::NotNeeded)?;
            }
        }
    }

    Ok(())
}

fn run_environment_build(
    build_process_supervisor: ChildProcessSupervisor,
) -> Result<EnvironmentBuildStatus> {
    match build_process_supervisor
        .run()
        .context("Environment building failed")?
    {
        ChildProcessOutcome::Exited(exit_status) => {
            if exit_status.success() {
                debug!("Environmenent building succeeded");
                Ok(EnvironmentBuildStatus::Success)
            } else {
                error!("Environment building not sucessful, suite will most likely not execute");
                Ok(EnvironmentBuildStatus::Failure)
            }
        }
        ChildProcessOutcome::TimedOut => {
            error!("Environment building timed out, suite will most likely not execute");
            Ok(EnvironmentBuildStatus::Timeout)
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Environment {
    System(SystemEnvironment),
    Rcc(RCCEnvironment),
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct SystemEnvironment {}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct RCCEnvironment {
    pub binary_path: Utf8PathBuf,
    pub robot_yaml_path: Utf8PathBuf,
    pub controller: String,
    pub space: String,
    pub build_timeout: u64,
}

impl Environment {
    pub fn new(suite_name: &str, environment_config: &EnvironmentConfig) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_environment_config.binary_path.clone(),
                robot_yaml_path: rcc_environment_config.robot_yaml_path.clone(),
                controller: String::from("robotmk"),
                space: suite_name.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
            }),
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        match self {
            Self::System(system_environment) => system_environment.wrap(command_spec),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command_spec),
        }
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match self {
            Self::System(_) => SystemEnvironment::create_result_code(exit_code),
            Self::Rcc(_) => RCCEnvironment::create_result_code(exit_code),
        }
    }

    fn build_instructions(&self) -> Option<BuildInstructions> {
        match self {
            Self::System(system_environment) => system_environment.build_instructions(),
            Self::Rcc(rcc_environment) => rcc_environment.build_instructions(),
        }
    }
}

pub enum ResultCode {
    AllTestsPassed,
    RobotCommandFailed,
    EnvironmentFailed,
}

struct BuildInstructions {
    command_spec: CommandSpec,
    timeout: u64,
}

impl SystemEnvironment {
    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        command_spec
    }

    fn create_result_code(exit_code: i32) -> ResultCode {
        if exit_code == 0 {
            return ResultCode::AllTestsPassed;
        }
        ResultCode::RobotCommandFailed
    }

    fn build_instructions(&self) -> Option<BuildInstructions> {
        None
    }
}

impl RCCEnvironment {
    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec = CommandSpec::new(&self.binary_path);
        wrapped_spec
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build");
        self.apply_current_settings(&mut wrapped_spec);
        wrapped_spec
            .add_argument("--")
            .add_argument(command_spec.executable)
            .add_arguments(command_spec.arguments);
        wrapped_spec
    }

    fn create_result_code(exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::AllTestsPassed,
            10 => ResultCode::RobotCommandFailed,
            _ => ResultCode::EnvironmentFailed,
        }
    }

    fn build_instructions(&self) -> Option<BuildInstructions> {
        let mut command_spec = CommandSpec::new(&self.binary_path);
        self.apply_current_settings(
            command_spec
                .add_argument("holotree")
                .add_argument("variables")
                .add_argument("--json"),
        );
        Some(BuildInstructions {
            command_spec,
            timeout: self.build_timeout,
        })
    }

    fn apply_current_settings(&self, command_spec: &mut CommandSpec) {
        command_spec
            .add_argument("--robot")
            .add_argument(&self.robot_yaml_path)
            .add_argument("--controller")
            .add_argument(&self.controller)
            .add_argument("--space")
            .add_argument(&self.space);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::external::RCCEnvironmentConfig;

    #[test]
    fn environment_from_system_config() {
        assert!(Environment::new("my_suite", &EnvironmentConfig::System)
            .build_instructions()
            .is_none())
    }

    #[test]
    fn environment_from_rcc_config() {
        assert!(Environment::new(
            "my_suite",
            &EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                build_timeout: 60,
            })
        )
        .build_instructions()
        .is_some())
    }

    fn command_spec_for_wrap() -> CommandSpec {
        let mut command_spec = CommandSpec::new("C:\\x\\y\\z.exe");
        command_spec
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        command_spec
    }

    #[test]
    fn test_system_wrap() {
        assert_eq!(
            SystemEnvironment {}.wrap(command_spec_for_wrap()),
            command_spec_for_wrap()
        );
    }

    #[test]
    fn test_rcc_wrap() {
        let mut expected = CommandSpec::new("C:\\bin\\z.exe");
        expected
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build")
            .add_argument("--robot")
            .add_argument("C:\\my_suite\\robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_suite")
            .add_argument("--")
            .add_argument("C:\\x\\y\\z.exe")
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                robot_yaml_path: Utf8PathBuf::from("C:\\my_suite\\robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_suite"),
                build_timeout: 600,
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }

    #[test]
    fn rcc_build_command() {
        let mut expected = CommandSpec::new("/bin/rcc");
        expected
            .add_argument("holotree")
            .add_argument("variables")
            .add_argument("--json")
            .add_argument("--robot")
            .add_argument("/a/b/c/robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_suite");

        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_suite"),
                build_timeout: 123,
            }
            .build_instructions()
            .unwrap()
            .command_spec,
            expected
        )
    }
}
