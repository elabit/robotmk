use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor};
use super::config::{Config, EnvironmentConfig};
use super::results::{EnvironmentBuildStatesAdministrator, EnvironmentBuildStatus};
use super::termination::TerminationFlag;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error, info};
use std::process::Command;

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
            Some(mut build_instructions) => {
                info!("Building environment for suite {}", suite_name);
                environment_build_states_administrator
                    .insert_and_write_atomic(suite_name, EnvironmentBuildStatus::InProgress)?;
                configure_stdio_of_environment_build(
                    &env_building_stdio_directory,
                    suite_name,
                    &mut build_instructions.command,
                )
                .context("Configuring stdio of environment build process failed")?;
                environment_build_states_administrator.insert_and_write_atomic(
                    suite_name,
                    run_environment_build(ChildProcessSupervisor {
                        command: build_instructions.command,
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

fn configure_stdio_of_environment_build(
    stdio_directory: &Utf8Path,
    suite_name: &str,
    build_command: &mut Command,
) -> Result<()> {
    let path_stdout = stdio_directory.join(format!("{}.stdout", suite_name));
    let path_stderr = stdio_directory.join(format!("{}.stderr", suite_name));
    build_command
        .stdout(
            std::fs::File::create(&path_stdout)
                .context(format!("Failed to open {path_stdout} for stdout capturing",))?,
        )
        .stderr(
            std::fs::File::create(&path_stderr)
                .context(format!("Failed to open {path_stderr} for stderr capturing",))?,
        );
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

pub enum Environment {
    System(SystemEnvironment),
    Rcc(RCCEnvironment),
}

pub struct SystemEnvironment {}

pub struct RCCEnvironment {
    binary_path: Utf8PathBuf,
    robocorp_home_path: Utf8PathBuf,
    robot_yaml_path: Utf8PathBuf,
    controller: String,
    space: String,
    build_timeout: u64,
}

impl Environment {
    pub fn new(suite_name: &str, environment_config: &EnvironmentConfig) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_environment_config.binary_path.clone(),
                robocorp_home_path: rcc_environment_config.robocorp_home_path.clone(),
                robot_yaml_path: rcc_environment_config.robot_yaml_path.clone(),
                controller: String::from("robotmk"),
                space: suite_name.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
            }),
        }
    }

    pub fn wrap(&self, command: Command) -> Command {
        match self {
            Self::System(system_environment) => system_environment.wrap(command),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command),
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
    command: Command,
    timeout: u64,
}

impl SystemEnvironment {
    fn wrap(&self, command: Command) -> Command {
        command
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
    fn wrap(&self, command: Command) -> Command {
        // TODO: Improve this
        if command.get_envs().len() > 0 {
            panic!("Currently unable to handle commands with environment variables");
        }
        let mut wrapped_cmd = Command::new(&self.binary_path);
        wrapped_cmd.arg("task").arg("script").arg("--no-build");
        self.apply_current_settings(&mut wrapped_cmd)
            .arg("--")
            .arg(command.get_program())
            .args(command.get_args());
        wrapped_cmd
    }

    fn create_result_code(exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::AllTestsPassed,
            10 => ResultCode::RobotCommandFailed,
            _ => ResultCode::EnvironmentFailed,
        }
    }

    fn build_instructions(&self) -> Option<BuildInstructions> {
        let mut build_cmd = Command::new(&self.binary_path);
        self.apply_current_settings(build_cmd.arg("holotree").arg("variables").arg("--json"));
        Some(BuildInstructions {
            command: build_cmd,
            timeout: self.build_timeout,
        })
    }

    fn apply_current_settings<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("ROBOCORP_HOME", &self.robocorp_home_path)
            .arg("--robot")
            .arg(&self.robot_yaml_path)
            .arg("--controller")
            .arg(&self.controller)
            .arg("--space")
            .arg(&self.space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RCCEnvironmentConfig;

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
                robocorp_home_path: Utf8PathBuf::from("/robocorp_home"),
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                build_timeout: 60,
            })
        )
        .build_instructions()
        .is_some())
    }

    fn command_for_wrap() -> Command {
        let mut command = Command::new("C:\\x\\y\\z.exe");
        command
            .arg("arg1")
            .arg("--flag")
            .arg("--option")
            .arg("option_value");
        command
    }

    #[test]
    fn test_system_wrap() {
        assert_eq!(
            format!("{:?}", SystemEnvironment {}.wrap(command_for_wrap()),),
            format!("{:?}", command_for_wrap())
        );
    }

    #[test]
    fn test_rcc_wrap() {
        let mut expected = Command::new("C:\\bin\\z.exe");
        expected
            .arg("task")
            .arg("script")
            .arg("--no-build")
            .arg("--robot")
            .arg("C:\\my_suite\\robot.yaml")
            .arg("--controller")
            .arg("robotmk")
            .arg("--space")
            .arg("my_suite")
            .arg("--")
            .arg("C:\\x\\y\\z.exe")
            .arg("arg1")
            .arg("--flag")
            .arg("--option")
            .arg("option_value")
            .env("ROBOCORP_HOME", "C:\\robocorp_home");

        assert_eq!(
            format!(
                "{:?}",
                RCCEnvironment {
                    binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                    robocorp_home_path: Utf8PathBuf::from("C:\\robocorp_home"),
                    robot_yaml_path: Utf8PathBuf::from("C:\\my_suite\\robot.yaml"),
                    controller: String::from("robotmk"),
                    space: String::from("my_suite"),
                    build_timeout: 600,
                }
                .wrap(command_for_wrap()),
            ),
            format!("{:?}", expected)
        );
    }

    #[test]
    fn rcc_build_command() {
        let mut expected = Command::new("/bin/rcc");
        expected
            .arg("holotree")
            .arg("variables")
            .arg("--json")
            .arg("--robot")
            .arg("/a/b/c/robot.yaml")
            .arg("--controller")
            .arg("robotmk")
            .arg("--space")
            .arg("my_suite")
            .env("ROBOCORP_HOME", "/robocorp_home");

        assert_eq!(
            format!(
                "{:?}",
                RCCEnvironment {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    robocorp_home_path: Utf8PathBuf::from("/robocorp_home"),
                    robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                    controller: String::from("robotmk"),
                    space: String::from("my_suite"),
                    build_timeout: 123,
                }
                .build_instructions()
                .unwrap()
                .command,
            ),
            format!("{:?}", expected)
        )
    }
}
