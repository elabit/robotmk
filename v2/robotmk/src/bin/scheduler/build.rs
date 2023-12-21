use super::internal_config::{GlobalConfig, Suite};
use super::logging::log_and_return_error;
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::{BuildOutcome, BuildStates, EnvironmentBuildStage};
use robotmk::section::WriteSection;
use robotmk::sessions::session::{RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

pub fn environment_building_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("environment_building")
}

pub fn build_environments(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> AnyhowResult<Vec<Suite>> {
    let mut build_stage_reporter = BuildStageReporter::new(
        suites.iter().map(|suite| suite.id.as_ref()),
        &global_config.results_directory,
        &global_config.results_directory_locker,
    )?;
    let working_directory =
        environment_building_working_directory(&global_config.working_directory);

    let mut completed_suites = Vec::new();
    for suite in suites.into_iter() {
        let outcome = build_environment(
            &suite.id,
            &suite.environment,
            &suite.session,
            &global_config.cancellation_token,
            &mut build_stage_reporter,
            &working_directory,
        )?;
        match outcome {
            BuildOutcome::NotNeeded => completed_suites.push(suite),
            BuildOutcome::Success(_) => completed_suites.push(suite),
            _ => {}
        }
    }
    Ok(completed_suites)
}

fn build_environment(
    id: &str,
    environment: &Environment,
    sesssion: &Session,
    cancellation_token: &CancellationToken,
    build_stage_reporter: &mut BuildStageReporter,
    working_directory: &Utf8Path,
) -> AnyhowResult<BuildOutcome> {
    let Some(build_instructions) = environment.build_instructions() else {
        let outcome = BuildOutcome::NotNeeded;
        info!("Nothing to do for suite {id}");
        build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
        return Ok(outcome);
    };
    info!("Building environment for suite {id}");
    let run_spec = RunSpec {
        id: &format!("robotmk_env_building_{id}"),
        command_spec: &build_instructions.command_spec,
        base_path: &working_directory.join(id),
        timeout: build_instructions.timeout,
        cancellation_token,
    };
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = run_build_command(&run_spec, sesssion, start_time).context(format!(
        "Received termination signal while building environment for suite {id}"
    ))?;
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
}

fn run_build_command(
    run_spec: &RunSpec,
    sesssion: &Session,
    reference_timestamp_for_duration: DateTime<Utc>,
) -> Result<BuildOutcome, Cancelled> {
    let outcome = match sesssion.run(run_spec) {
        Ok(o) => o,
        Err(e) => {
            let e = e.context("Environment building failed, suite will be dropped");
            let e = log_and_return_error(e);
            return Ok(BuildOutcome::Error(format!("{e:?}")));
        }
    };
    let duration = (Utc::now() - reference_timestamp_for_duration).num_seconds();
    let exit_code = match outcome {
        Outcome::Completed(exit_code) => exit_code,
        Outcome::Timeout => {
            error!("Environment building timed out, suite will be dropped");
            return Ok(BuildOutcome::Timeout);
        }
        Outcome::Cancel => {
            error!("Environment building cancelled");
            return Err(Cancelled {});
        }
    };
    match exit_code.context("Failed to retrieve exit code of environment build process") {
        Ok(0) => {
            info!("Environmenent building succeeded");
            Ok(BuildOutcome::Success(duration))
        }
        Ok(_) => {
            error!("Environment building not sucessful, suite will be dropped");
            Ok(BuildOutcome::Error(
                "Environment building not sucessful, see stdio logs".into(),
            ))
        }
        Err(error) => {
            error!("Suite will be dropped: {error:?}");
            Ok(BuildOutcome::Error(format!("{error:?}")))
        }
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
    ) -> AnyhowResult<BuildStageReporter<'a>> {
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

    pub fn update(
        &mut self,
        suite_id: &str,
        build_status: EnvironmentBuildStage,
    ) -> AnyhowResult<()> {
        self.build_states.insert(suite_id.into(), build_status);
        BuildStates(&self.build_states).write(&self.path, self.locker)
    }
}
