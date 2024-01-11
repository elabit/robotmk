use crate::internal_config::Suite;
use crate::logging::TIMESTAMP_FORMAT;
use robotmk::results::{AttemptsConfig, SuiteExecutionReport};
use robotmk::suites::run_attempts_with_rebot;

use anyhow::{Context, Result as AnyhowResult};
use chrono::Utc;
use log::info;
use robotmk::section::WritePiggybackSection;
use std::fs::create_dir_all;

pub fn run_suite(suite: &Suite) -> AnyhowResult<()> {
    info!("Running suite {}", &suite.id);
    produce_suite_results(suite)?
        .write(
            &suite.results_file,
            suite.host.clone(),
            &suite.results_directory_locker,
        )
        .context("Reporting suite results failed")?;
    info!("Suite {} finished", &suite.id);

    Ok(())
}

fn produce_suite_results(suite: &Suite) -> AnyhowResult<SuiteExecutionReport> {
    let timestamp = Utc::now();
    let output_directory = suite
        .working_directory
        .join(timestamp.format(TIMESTAMP_FORMAT).to_string());

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for suite run: {}",
        output_directory
    ))?;

    let (attempt_reports, rebot) = run_attempts_with_rebot(
        &suite.robot,
        &suite.id,
        &suite.environment,
        &suite.session,
        suite.timeout,
        &suite.cancellation_token,
        &output_directory,
    )
    .context("Received termination signal while running suite")?;

    Ok(SuiteExecutionReport {
        suite_id: suite.id.clone(),
        timestamp: timestamp.timestamp(),
        attempts: attempt_reports,
        rebot,
        config: AttemptsConfig {
            interval: suite.group_affiliation.execution_interval,
            timeout: suite.timeout,
            n_attempts_max: suite.robot.n_attempts_max,
        },
        metadata: suite.metadata.clone(),
    })
}
