use super::internal_config::{GlobalConfig, Suite};
use super::logging::log_and_return_error;
use robotmk::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use robotmk::command_spec::CommandSpec;
use robotmk::config::EnvironmentConfig;
use robotmk::environment::ResultCode;
use robotmk::results::{BuildOutcome, BuildStates, EnvironmentBuildStage};

use robotmk::lock::Locker;
use robotmk::section::WriteSection;

use anyhow::{bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

pub fn environment_building_stdio_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("environment_building_stdio")
}

pub fn build_environments(global_config: &GlobalConfig, suites: Vec<Suite>) -> Result<Vec<Suite>> {
    let mut build_stage_reporter = BuildStageReporter::new(
        suites.iter().map(|suite| suite.id.as_ref()),
        &global_config.results_directory,
        &global_config.results_directory_locker,
    )?;
    let env_building_stdio_directory =
        environment_building_stdio_directory(&global_config.working_directory);

    let mut completed_suites = Vec::new();
    for suite in suites.into_iter() {
        let outcome = build_environment(
            &suite.id,
            suite.environment.build_instructions(),
            &global_config.cancellation_token,
            &mut build_stage_reporter,
            &env_building_stdio_directory,
        )?;
        match outcome {
            BuildOutcome::NotNeeded => completed_suites.push(suite),
            BuildOutcome::Success(_) => completed_suites.push(suite),
            BuildOutcome::Terminated => bail!("Terminated"),
            _ => {}
        }
    }
    Ok(completed_suites)
}

fn build_environment(
    id: &str,
    instructions: Option<BuildInstructions>,
    cancellation_token: &CancellationToken,
    build_stage_reporter: &mut BuildStageReporter,
    stdio_directory: &Utf8Path,
) -> Result<BuildOutcome> {
    let Some(instructions) = instructions else {
        let outcome = BuildOutcome::NotNeeded;
        debug!("Nothing to do for suite {}", id);
        build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
        return Ok(outcome);
    };
    info!("Building environment for suite {}", id);
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = run_environment_build(
        ChildProcessSupervisor {
            command_spec: &instructions.command_spec,
            stdio_paths: Some(StdioPaths {
                stdout: stdio_directory.join(format!("{}.stdout", id)),
                stderr: stdio_directory.join(format!("{}.stderr", id)),
            }),
            timeout: instructions.timeout,
            cancellation_token,
        },
        start_time,
    )?;
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
}

fn run_environment_build(
    build_process_supervisor: ChildProcessSupervisor,
    reference_timestamp_for_duration: DateTime<Utc>,
) -> Result<BuildOutcome> {
    let build_result = build_process_supervisor.run();
    let child_process_outcome = match build_result {
        Ok(o) => o,
        Err(e) => {
            let e = e.context("Environment building failed, suite will be dropped");
            let e = log_and_return_error(e);
            return Ok(BuildOutcome::Error(format!("{e:?}")));
        }
    };
    let duration = (Utc::now() - reference_timestamp_for_duration).num_seconds();
    match child_process_outcome {
        ChildProcessOutcome::Exited(exit_status) => {
            if exit_status.success() {
                debug!("Environmenent building succeeded");
                Ok(BuildOutcome::Success(duration))
            } else {
                error!("Environment building not sucessful, suite will be dropped");
                Ok(BuildOutcome::NonZeroExit)
            }
        }
        ChildProcessOutcome::TimedOut => {
            error!("Environment building timed out, suite will be dropped");
            Ok(BuildOutcome::Timeout)
        }
        ChildProcessOutcome::Terminated => Ok(BuildOutcome::Terminated),
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
    pub env_json_path: Option<Utf8PathBuf>,
}

impl Environment {
    pub fn new(
        suite_id: &str,
        rcc_binary_path: &Utf8Path,
        environment_config: &EnvironmentConfig,
    ) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_binary_path.to_path_buf(),
                robot_yaml_path: rcc_environment_config.robot_yaml_path.clone(),
                controller: String::from("robotmk"),
                space: suite_id.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
                env_json_path: rcc_environment_config.env_json_path.clone(),
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
        if let Some(env_json_path) = &self.env_json_path {
            command_spec
                .add_argument("--environment")
                .add_argument(env_json_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robotmk::config::RCCEnvironmentConfig;

    #[test]
    fn environment_from_system_config() {
        assert!(
            Environment::new("my_suite", "/bin/rcc".into(), &EnvironmentConfig::System)
                .build_instructions()
                .is_none()
        )
    }

    #[test]
    fn environment_from_rcc_config() {
        assert!(Environment::new(
            "my_suite",
            "/bin/rcc".into(),
            &EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                build_timeout: 60,
                env_json_path: None,
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
            .add_argument("--environment")
            .add_argument("C:\\my_suite\\env.json")
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
                env_json_path: Some("C:\\my_suite\\env.json".into())
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
                env_json_path: None,
            }
            .build_instructions()
            .unwrap()
            .command_spec,
            expected
        )
    }
}

struct BuildStageReporter<'a> {
    build_states: HashMap<String, EnvironmentBuildStage>,
    path: Utf8PathBuf,
    locker: &'a Locker,
}

impl<'a> BuildStageReporter<'a> {
    pub fn new<'c>(
        ids: impl Iterator<Item = &'c str>,
        results_directory: &Utf8Path,
        locker: &'a Locker,
    ) -> Result<BuildStageReporter<'a>> {
        let build_states: HashMap<_, _> = ids
            .map(|id| (id.to_string(), EnvironmentBuildStage::Pending))
            .collect();
        let path = results_directory.join("environment_build_states.json");
        BuildStates(&build_states).write(&path, locker)?;
        Ok(Self {
            build_states,
            path,
            locker,
        })
    }

    pub fn update(&mut self, suite_id: &str, build_status: EnvironmentBuildStage) -> Result<()> {
        self.build_states.insert(suite_id.into(), build_status);
        BuildStates(&self.build_states).write(&self.path, self.locker)
    }
}
