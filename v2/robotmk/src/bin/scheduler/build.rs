use super::internal_config::{GlobalConfig, Suite};
use super::logging::log_and_return_error;
use robotmk::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use robotmk::command_spec::CommandSpec;
use robotmk::environment::{apply_current_settings, Environment, RCCEnvironment};
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
            &suite.environment,
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
    environment: &Environment,
    cancellation_token: &CancellationToken,
    build_stage_reporter: &mut BuildStageReporter,
    stdio_directory: &Utf8Path,
) -> Result<BuildOutcome> {
    let Environment::Rcc(environment) = environment else {
        let outcome = BuildOutcome::NotNeeded;
        debug!("Nothing to do for suite {}", id);
        build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
        return Ok(outcome);
    };
    info!("Building environment for suite {}", id);
    let command_spec = create_build_command(environment);
    let supervisor = ChildProcessSupervisor {
        command_spec: &command_spec,
        stdio_paths: Some(StdioPaths {
            stdout: stdio_directory.join(format!("{}.stdout", id)),
            stderr: stdio_directory.join(format!("{}.stderr", id)),
        }),
        timeout: environment.build_timeout,
        cancellation_token,
    };
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = run_build_command(supervisor, start_time)?;
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
}

fn create_build_command(environment: &RCCEnvironment) -> CommandSpec {
    let mut command_spec = CommandSpec::new(&environment.binary_path);
    command_spec
        .add_argument("holotree")
        .add_argument("variables")
        .add_argument("--json");
    apply_current_settings(
        &environment.robot_yaml_path,
        &environment.controller,
        &environment.space,
        environment.env_json_path.as_deref(),
        &mut command_spec,
    );
    command_spec
}

fn run_build_command(
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

#[cfg(test)]
mod tests {
    use super::*;

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
            create_build_command(&RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_suite"),
                build_timeout: 123,
                env_json_path: None,
            }),
            expected
        )
    }
}
