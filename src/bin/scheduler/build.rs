use super::internal_config::{GlobalConfig, Plan};
use robotmk::environment::{BuildInstructions, Environment};
use robotmk::lock::Locker;
use robotmk::results::{BuildOutcome, BuildStates, EnvironmentBuildStage};
use robotmk::section::WriteSection;
use robotmk::session::{RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome, Terminate};

use anyhow::anyhow;
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
    plans: Vec<Plan>,
) -> Result<Vec<Plan>, Terminate> {
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
            BuildOutcome::NotNeeded | BuildOutcome::Success(_) => completed_plans.push(plan),
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
) -> Result<BuildOutcome, Terminate> {
    let Some(build_instructions) = environment.build_instructions() else {
        let outcome = BuildOutcome::NotNeeded;
        info!("Nothing to do for plan {id}");
        build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
        return Ok(outcome);
    };
    let base_path = &working_directory.join(session.id()).join(id);
    info!("Building environment for plan {id}");
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = run_build_commands(
        id,
        &build_instructions,
        session,
        start_time,
        cancellation_token,
        base_path,
    )?;
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
}

fn run_build_commands(
    id: &str,
    build_instructions: &BuildInstructions,
    session: &Session,
    start_time: DateTime<Utc>,
    cancellation_token: &CancellationToken,
    base_path: &Utf8Path,
) -> Result<BuildOutcome, Cancelled> {
    if let Some(command_spec) = &build_instructions.import_command_spec {
        let import_run_spec = RunSpec {
            id: &format!("robotmk_env_import_{id}"),
            command_spec,
            base_path,
            timeout: build_instructions.timeout,
            cancellation_token,
        };
        match session.run(&import_run_spec) {
            Ok(Outcome::Completed(0)) => {
                info!("Environment import succeeded for plan {id}");
            }
            Ok(Outcome::Completed(_exit_code)) => {
                error!("Environment import not successful, plan {id} will be dropped");
                return Ok(BuildOutcome::Error(format!(
                    "Environment import not successful, see {base_path} for stdio logs"
                )));
            }
            Ok(Outcome::Timeout) => {
                error!("Environment import timed out, plan {id} will be dropped");
                return Ok(BuildOutcome::Timeout);
            }
            Ok(Outcome::Cancel) => {
                error!("Environment import cancelled, plan {id} will be dropped");
                return Err(Cancelled {});
            }
            Err(e) => {
                let log_error = e.context(anyhow!(
                    "Environment import failed, plan {id} will be dropped. See {} for stdio logs",
                    base_path,
                ));
                error!("{log_error:?}");
                return Ok(BuildOutcome::Error(format!("{log_error:?}")));
            }
        };
    } else {
        info!("No holotree zip. Environment import skipped.");
    };
    let elapsed: u64 = (Utc::now() - start_time).num_seconds().try_into().unwrap();
    if elapsed >= build_instructions.timeout {
        error!("Environment import timed out, plan {id} will be dropped");
        return Ok(BuildOutcome::Timeout);
    };
    let build_run_spec = RunSpec {
        id: &format!("robotmk_env_building_{id}"),
        command_spec: &build_instructions.build_command_spec,
        base_path,
        timeout: build_instructions.timeout - elapsed,
        cancellation_token,
    };
    match session.run(&build_run_spec) {
        Ok(Outcome::Completed(0)) => {
            info!("Environment building succeeded for plan {id}");
            let duration = (Utc::now() - start_time).num_seconds();
            Ok(BuildOutcome::Success(duration))
        }
        Ok(Outcome::Completed(_exit_code)) => {
            error!("Environment building not successful, plan {id} will be dropped");
            Ok(BuildOutcome::Error(format!(
                "Environment building not successful, see {base_path} for stdio logs",
            )))
        }
        Ok(Outcome::Timeout) => {
            error!("Environment building timed out, plan {id} will be dropped");
            Ok(BuildOutcome::Timeout)
        }
        Ok(Outcome::Cancel) => {
            error!("Environment building cancelled, plan {id} will be dropped");
            Err(Cancelled {})
        }
        Err(e) => {
            let log_error = e.context(anyhow!(
                "Environment building failed, plan {id} will be dropped. See {} for stdio logs",
                base_path,
            ));
            error!("{log_error:?}");
            Ok(BuildOutcome::Error(format!("{log_error:?}")))
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
    ) -> Result<BuildStageReporter<'a>, Terminate> {
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
    ) -> Result<(), Terminate> {
        self.build_states.insert(plan_id.into(), build_status);
        BuildStates(&self.build_states).write(&self.path, self.locker)
    }
}
