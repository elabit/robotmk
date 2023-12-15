use crate::internal_config::Suite;
use crate::logging::TIMESTAMP_FORMAT;
use robotmk::results::{AttemptsConfig, SuiteExecutionReport};
use robotmk::suites::run_attempts_with_rebot;

use anyhow::{Context, Result};
use chrono::Utc;
use log::debug;
use robotmk::section::WritePiggybackSection;
use std::fs::create_dir_all;

pub fn run_suite(suite: &Suite) -> Result<()> {
    debug!("Running suite {}", &suite.id);
    produce_suite_results(suite)?
        .write(
            &suite.results_file,
            suite.host.clone(),
            &suite.results_directory_locker,
        )
        .context("Reporting suite results failed")?;
    debug!("Suite {} finished", &suite.id);

    Ok(())
}

fn produce_suite_results(suite: &Suite) -> Result<SuiteExecutionReport> {
    let timestamp = Utc::now().format(TIMESTAMP_FORMAT).to_string();
    let output_directory = suite.working_directory.join(timestamp.clone());

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for suite run: {}",
        output_directory
    ))?;

    let (attempt_outcomes, rebot) = run_attempts_with_rebot(
        &suite.robot,
        &suite.id,
        &suite.environment,
        &suite.session,
        suite.timeout,
        &suite.cancellation_token,
        &output_directory,
    )?;

    Ok(SuiteExecutionReport {
        suite_id: suite.id.clone(),
        timestamp,
        attempts: attempt_outcomes,
        rebot,
        config: AttemptsConfig {
            interval: suite.execution_interval_seconds,
            timeout: suite.timeout,
            n_attempts_max: suite.robot.n_attempts_max,
        },
    })
}
