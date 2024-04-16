use crate::internal_config::Plan;
use crate::logging::TIMESTAMP_FORMAT;
use robotmk::results::{AttemptsConfig, PlanExecutionReport};
use robotmk::suites::run_attempts_with_rebot;

use anyhow::{Context, Result as AnyhowResult};
use chrono::Utc;
use log::info;
use robotmk::section::WritePiggybackSection;
use std::fs::create_dir_all;

pub fn run_plan(plan: &Plan) -> AnyhowResult<()> {
    info!("Running plan {}", &plan.id);
    produce_plan_results(plan)?
        .write(
            &plan.results_file,
            plan.host.clone(),
            &plan.results_directory_locker,
        )
        .context("Reporting plan results failed")?;
    info!("Plan {} finished", &plan.id);

    Ok(())
}

fn produce_plan_results(plan: &Plan) -> AnyhowResult<PlanExecutionReport> {
    let timestamp = Utc::now();
    let output_directory = plan
        .working_directory
        .join(timestamp.format(TIMESTAMP_FORMAT).to_string());

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for plan run: {}",
        output_directory
    ))?;

    let (attempt_reports, rebot) = run_attempts_with_rebot(
        &plan.robot,
        &plan.id,
        &plan.environment,
        &plan.session,
        plan.timeout,
        &plan.cancellation_token,
        &output_directory,
    )
    .context("Received termination signal while running plan")?;

    Ok(PlanExecutionReport {
        plan_id: plan.id.clone(),
        timestamp: timestamp.timestamp(),
        attempts: attempt_reports,
        rebot,
        config: AttemptsConfig {
            interval: plan.group_affiliation.execution_interval,
            timeout: plan.timeout,
            n_attempts_max: plan.robot.n_attempts_max,
        },
        metadata: plan.metadata.clone(),
    })
}
