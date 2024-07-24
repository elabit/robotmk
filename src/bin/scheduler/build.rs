use super::internal_config::{GlobalConfig, Plan};
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::{BuildOutcome, BuildStates, EnvironmentBuildStage};
use robotmk::section::{WriteError, WriteSection};
use robotmk::session::{RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Failed to `{0}`")]
    Unrecoverable(String),
    #[error("Terminated")]
    Cancelled,
}

impl From<WriteError> for BuildError {
    fn from(value: WriteError) -> Self {
        match value {
            WriteError::Cancelled => Self::Cancelled,
            value => Self::Unrecoverable(format!("{:?}", value)),
        }
    }
}

impl From<anyhow::Error> for BuildError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unrecoverable(format!("{:?}", value))
    }
}

impl From<Cancelled> for BuildError {
    fn from(_value: Cancelled) -> Self {
        Self::Cancelled
    }
}

pub fn environment_building_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("environment_building")
}

pub fn build_environments(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<Vec<Plan>, BuildError> {
    let mut build_stage_reporter = BuildStageReporter::new(
        plans.iter().map(|plan| plan.id.as_ref()),
        &global_config.results_directory,
        &global_config.results_directory_locker,
    )?;
    let working_directory =
        environment_building_working_directory(&global_config.working_directory);

    let mut completed_plans = Vec::new();
    for plan in plans.into_iter() {
        let outcome = build_environment(
            &plan.id,
            &plan.environment,
            &plan.session,
            &global_config.cancellation_token,
            &mut build_stage_reporter,
            &working_directory,
        )?;
        match outcome {
            BuildOutcome::NotNeeded => completed_plans.push(plan),
            BuildOutcome::Success(_) => completed_plans.push(plan),
            _ => {}
        }
    }
    Ok(completed_plans)
}

fn build_environment(
    id: &str,
    environment: &Environment,
    session: &Session,
    cancellation_token: &CancellationToken,
    build_stage_reporter: &mut BuildStageReporter,
    working_directory: &Utf8Path,
) -> Result<BuildOutcome, BuildError> {
    let Some(build_instructions) = environment.build_instructions() else {
        let outcome = BuildOutcome::NotNeeded;
        info!("Nothing to do for plan {id}");
        build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
        return Ok(outcome);
    };
    let base_path = &working_directory.join(session.id()).join(id);
    info!("Building environment for plan {id}");
    let run_spec = RunSpec {
        id: &format!("robotmk_env_building_{id}"),
        command_spec: &build_instructions.command_spec,
        base_path,
        timeout: build_instructions.timeout,
        cancellation_token,
    };
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = run_build_command(id, &run_spec, session, start_time)?;
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
}

fn run_build_command(
    id: &str,
    run_spec: &RunSpec,
    session: &Session,
    reference_timestamp_for_duration: DateTime<Utc>,
) -> Result<BuildOutcome, Cancelled> {
    let outcome = match session.run(run_spec) {
        Ok(o) => o,
        Err(e) => {
            let log_error = e.context(anyhow!(
                "Environment building failed, plan {id} will be dropped. See {} for stdio logs",
                run_spec.base_path,
            ));
            error!("{log_error:?}");
            return Ok(BuildOutcome::Error(format!("{log_error:?}")));
        }
    };
    let duration = (Utc::now() - reference_timestamp_for_duration).num_seconds();
    let exit_code = match outcome {
        Outcome::Completed(exit_code) => exit_code,
        Outcome::Timeout => {
            error!("Environment building timed out, plan {id} will be dropped");
            return Ok(BuildOutcome::Timeout);
        }
        Outcome::Cancel => {
            error!("Environment building cancelled, plan {id} will be dropped");
            return Err(Cancelled {});
        }
    };
    if exit_code == 0 {
        info!("Environment building succeeded for plan {id}");
        Ok(BuildOutcome::Success(duration))
    } else {
        error!("Environment building not successful, plan {id} will be dropped");
        Ok(BuildOutcome::Error(format!(
            "Environment building not successful, see {} for stdio logs",
            run_spec.base_path
        )))
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
    ) -> Result<BuildStageReporter<'a>, WriteError> {
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
        plan_id: &str,
        build_status: EnvironmentBuildStage,
    ) -> Result<(), WriteError> {
        self.build_states.insert(plan_id.into(), build_status);
        BuildStates(&self.build_states).write(&self.path, self.locker)
    }
}
