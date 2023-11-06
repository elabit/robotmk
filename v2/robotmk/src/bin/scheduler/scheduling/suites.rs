use crate::environment::ResultCode;
use crate::internal_config::Suite;
use crate::results::{
    AttemptOutcome, AttemptsOutcome, ExecutionReport, SuiteExecutionReport, WriteSection,
};
use crate::rf::{rebot::Rebot, robot::Attempt};
use crate::sessions::session::{RunOutcome, RunSpec};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::{debug, error};
use std::fs::create_dir_all;
use std::sync::{MutexGuard, TryLockError};

pub fn run_suite(suite: &Suite) -> Result<()> {
    // We hold the lock as long as `_non_parallel_guard` is in scope
    let _non_parallel_guard = try_acquire_suite_lock(suite).map_err(|err| {
        let report = SuiteExecutionReport {
            suite_name: suite.name.clone(),
            outcome: ExecutionReport::AlreadyRunning,
        };
        report
            .write(&suite.results_file)
            .context("Reporting failure to acquire suite lock failed")
            .err()
            .unwrap_or(err)
    })?;

    debug!("Running suite {}", &suite.name);
    let report = SuiteExecutionReport {
        suite_name: suite.name.clone(),
        outcome: ExecutionReport::Executed(produce_suite_results(suite)?),
    };
    report
        .write(&suite.results_file)
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
    let output_directory = suite
        .working_directory
        .join(Utc::now().format("%Y-%m-%dT%H.%M.%S%.f%z").to_string());

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for suite run: {}",
        output_directory
    ))?;

    let (attempt_outcomes, output_paths) = run_attempts_until_succesful(suite, &output_directory)?;

    Ok(AttemptsOutcome {
        attempts: attempt_outcomes,
        rebot: if output_paths.is_empty() {
            None
        } else {
            Some(
                Rebot {
                    environment: &suite.environment,
                    input_paths: &output_paths,
                    path_xml: &output_directory.join("rebot.xml"),
                    path_html: &output_directory.join("rebot.html"),
                }
                .rebot(),
            )
        },
    })
}

fn run_attempts_until_succesful(
    suite: &Suite,
    output_directory: &Utf8Path,
) -> Result<(Vec<AttemptOutcome>, Vec<Utf8PathBuf>)> {
    let mut outcomes = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in suite.robot.attempts(output_directory) {
        let (outcome, output_path) = run_attempt(suite, attempt, output_directory)?;
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

fn run_attempt(
    suite: &Suite,
    attempt: Attempt,
    output_directory: &Utf8Path,
) -> Result<(AttemptOutcome, Option<Utf8PathBuf>)> {
    let log_message_start = format!("Suite {}, attempt {}", suite.name, attempt.index);

    let run_outcome = match suite.session.run(&RunSpec {
        id: &format!("robotmk_suite_{}_attempt_{}", suite.name, attempt.index),
        command_spec: &suite.environment.wrap(attempt.command_spec),
        base_path: &output_directory.join(attempt.index.to_string()),
        timeout: suite.timeout,
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
                Some(attempt.output_xml_file),
            ))
        }
        ResultCode::EnvironmentFailed => {
            error!("{log_message_start}: environment failure");
            Ok((AttemptOutcome::EnvironmentFailure, None))
        }
        ResultCode::RobotCommandFailed => {
            if attempt.output_xml_file.exists() {
                debug!("{log_message_start}: some tests failed");
                Ok((AttemptOutcome::TestFailures, Some(attempt.output_xml_file)))
            } else {
                error!("{log_message_start}: Robot Framework failure (no output)");
                Ok((AttemptOutcome::RobotFrameworkFailure, None))
            }
        }
    }
}
