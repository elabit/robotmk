use super::internal_config::{GlobalConfig, Plan};
use robotmk::env::Environment;
use robotmk::lock::Locker;
use robotmk::results::{BuildOutcome, BuildStates, EnvironmentBuildStage};
use robotmk::section::WriteSection;
use robotmk::session::Session;
use robotmk::termination::Terminate;

use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::info;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

pub fn build_environments(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<Vec<Plan>, Terminate> {
    let mut build_stage_reporter = BuildStageReporter::new(
        plans.iter().map(|plan| plan.id.as_ref()),
        &global_config.results_directory,
        &global_config.results_directory_locker,
    )?;
    let mut completed_plans = Vec::new();
    for plan in plans.into_iter() {
        let outcome = build_environment(
            &plan.id,
            &plan.environment,
            &plan.session,
            &global_config.cancellation_token,
            &mut build_stage_reporter,
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
) -> Result<BuildOutcome, Terminate> {
    info!("Processing plan {id}");
    let start_time = Utc::now();
    build_stage_reporter.update(
        id,
        EnvironmentBuildStage::InProgress(start_time.timestamp()),
    )?;
    let outcome = environment.build(id, session, start_time, cancellation_token)?;
    if let BuildOutcome::NotNeeded = outcome {
        info!("Nothing to do for plan {id}");
    }
    build_stage_reporter.update(id, EnvironmentBuildStage::Complete(outcome.clone()))?;
    Ok(outcome)
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
