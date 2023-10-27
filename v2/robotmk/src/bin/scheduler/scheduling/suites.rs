use crate::config::internal::Suite;
use crate::environment::ResultCode;
use crate::rebot::Rebot;
use crate::results::{
    write_file_atomic, AttemptOutcome, AttemptsOutcome, ExecutionReport, SuiteExecutionReport,
};
use crate::robot::{Attempt, Identifier, RetrySpec};
use crate::sessions::session::{RunOutcome, RunSpec};

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use chrono::Utc;
use log::{debug, error};
use serde_json::to_string;
use std::fs::create_dir_all;
use std::sync::{MutexGuard, TryLockError};

pub fn run_suite(suite: &Suite) -> Result<()> {
    // We hold the lock as long as `_non_parallel_guard` is in scope
    let _non_parallel_guard = try_acquire_suite_lock(suite).map_err(|err| {
        persist_suite_execution_report(
            suite,
            &SuiteExecutionReport {
                suite_name: suite.name.clone(),
                outcome: ExecutionReport::AlreadyRunning,
            },
        )
        .context("Reporting failure to acquire suite lock failed")
        .err()
        .unwrap_or(err)
    })?;

    debug!("Running suite {}", &suite.name);
    persist_suite_execution_report(
        suite,
        &SuiteExecutionReport {
            suite_name: suite.name.clone(),
            outcome: ExecutionReport::Executed(produce_suite_results(suite)?),
        },
    )
    .context("Reporting suite results failed")?;
    debug!("Suite {} finished", &suite.name);

    Ok(())
}

pub fn try_acquire_suite_lock(suite: &Suite) -> Result<MutexGuard<usize>> {
    match suite.parallelism_protection.try_lock() {
        Ok(non_parallel_guard) => Ok(non_parallel_guard),
        Err(try_lock_error) => match try_lock_error {
            TryLockError::WouldBlock => {
                bail!(
                    "Failed to acquire lock for suite {}, skipping this run",
                    suite.name
                );
            }
            TryLockError::Poisoned(poison_error) => {
                error!("Lock for suite {} poisoned, unpoisoning", suite.name);
                Ok(poison_error.into_inner())
            }
        },
    }
}

fn produce_suite_results(suite: &Suite) -> Result<AttemptsOutcome> {
    let retry_spec = RetrySpec {
        identifier: Identifier {
            name: &suite.name,
            timestamp: Utc::now().format("%Y-%m-%dT%H.%M.%S%.f%z").to_string(),
        },
        working_directory: &suite.working_directory,
        execution_config: &suite.execution_config,
        robot_framework_config: &suite.robot_framework_config,
    };

    create_dir_all(retry_spec.output_directory()).context(format!(
        "Failed to create directory for suite run: {}",
        retry_spec.output_directory()
    ))?;

    let (attempt_outcomes, output_paths) =
        run_attempts_until_succesful(retry_spec.attempts(), suite)?;

    Ok(AttemptsOutcome {
        attempts: attempt_outcomes,
        rebot: if output_paths.is_empty() {
            None
        } else {
            Some(
                Rebot {
                    environment: &suite.environment,
                    input_paths: &output_paths,
                    path_xml: &retry_spec.output_directory().join("rebot.xml"),
                    path_html: &retry_spec.output_directory().join("rebot.html"),
                }
                .rebot(),
            )
        },
    })
}

fn run_attempts_until_succesful<'a>(
    attempts: impl Iterator<Item = Attempt<'a>>,
    suite: &Suite,
) -> Result<(Vec<AttemptOutcome>, Vec<Utf8PathBuf>)> {
    let mut outcomes = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in attempts {
        let (outcome, output_path) = run_attempt(&attempt, suite)?;
        let success = matches!(&outcome, &AttemptOutcome::AllTestsPassed);
        outcomes.push(outcome);
        if let Some(output_path) = output_path {
            output_paths.push(output_path);
        }
        if success {
            break;
        }
    }

    Ok((outcomes, output_paths))
}

fn run_attempt(attempt: &Attempt, suite: &Suite) -> Result<(AttemptOutcome, Option<Utf8PathBuf>)> {
    let log_message_start = format!(
        "Suite {}, attempt {}",
        attempt.identifier.name, attempt.index
    );

    let run_outcome = match suite.session.run(&RunSpec {
        id: &format!(
            "robotmk_suite_{}_attempt_{}",
            attempt.identifier.name, attempt.index,
        ),
        command_spec: &suite.environment.wrap(attempt.command_spec()),
        base_path: &attempt.output_directory.join(attempt.index.to_string()),
        timeout: attempt.timeout,
        termination_flag: &suite.termination_flag,
    }) {
        Ok(run_outcome) => run_outcome,
        Err(error_) => {
            error!("{log_message_start}: {error_:?}");
            return Ok((AttemptOutcome::OtherError(format!("{error_:?}")), None));
        }
    };
    let exit_code = match run_outcome {
        RunOutcome::Exited(exit_code) => exit_code,
        RunOutcome::TimedOut => {
            error!("{log_message_start}: timed out");
            return Ok((AttemptOutcome::TimedOut, None));
        }
        RunOutcome::Terminated => bail!("Terminated"),
    };
    let exit_code = match exit_code {
        Some(exit_code) => exit_code,
        None => {
            error!("{log_message_start}: failed to query exit code");
            return Ok((
                AttemptOutcome::OtherError(
                    "Failed to query exit code of Robot Framework call".into(),
                ),
                None,
            ));
        }
    };
    match suite.environment.create_result_code(exit_code) {
        ResultCode::AllTestsPassed => {
            debug!("{log_message_start}: all tests passed");
            Ok((
                AttemptOutcome::AllTestsPassed,
                Some(attempt.output_xml_file()),
            ))
        }
        ResultCode::EnvironmentFailed => {
            error!("{log_message_start}: environment failure");
            Ok((AttemptOutcome::EnvironmentFailure, None))
        }
        ResultCode::RobotCommandFailed => {
            if attempt.output_xml_file().exists() {
                debug!("{log_message_start}: some tests failed");
                Ok((
                    AttemptOutcome::TestFailures,
                    Some(attempt.output_xml_file()),
                ))
            } else {
                error!("{log_message_start}: Robot Framework failure (no output)");
                Ok((AttemptOutcome::RobotFrameworkFailure, None))
            }
        }
    }
}

fn persist_suite_execution_report(
    suite: &Suite,
    suite_execution_report: &SuiteExecutionReport,
) -> Result<()> {
    write_file_atomic(
        &to_string(suite_execution_report).context("Serializing suite execution report failed")?,
        &suite.results_file,
    )
}
