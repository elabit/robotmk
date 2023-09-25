use super::attempt::{Attempt, Identifier, RetrySpec};
use super::config::{Config, SuiteConfig};
use super::environment::{Environment, ResultCode};
use super::logging::log_and_return_error;
use super::rebot::Rebot;
use super::results::{
    suite_result_file, suite_results_directory, write_file_atomic, AttemptOutcome, AttemptsOutcome,
    ExecutionReport, SuiteExecutionReport,
};
use super::session::{RunOutcome, Session};
use super::termination::TerminationFlag;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clokwerk::{Scheduler, TimeUnits};
use log::{debug, error};
use serde_json::to_string;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread::{sleep, spawn};
use std::time::Duration;

pub fn run_suites(config: &Config, termination_flag: &TerminationFlag) -> Result<()> {
    let mut scheduler = Scheduler::new();

    for (suite_name, suite_config) in config.suites() {
        let suite_run_spec = Arc::new(SuiteRunSpec {
            termination_flag: termination_flag.clone(),
            parallelism_protection: Mutex::new(0),
            suite_name: suite_name.clone(),
            suite_config: suite_config.clone(),
            working_directory: config.working_directory.clone(),
            result_file: suite_result_file(
                &suite_results_directory(&config.results_directory),
                suite_name,
            ),
        });
        scheduler
            .every(
                suite_config
                    .execution_config
                    .execution_interval_seconds
                    .seconds(),
            )
            .run(move || run_suite_in_new_thread(suite_run_spec.clone()));
    }

    loop {
        if termination_flag.should_terminate() {
            bail!("Terminated");
        }
        scheduler.run_pending();
        sleep(Duration::from_millis(250));
    }
}

struct SuiteRunSpec {
    termination_flag: TerminationFlag,
    parallelism_protection: Mutex<usize>,
    suite_name: String,
    suite_config: SuiteConfig,
    working_directory: PathBuf,
    result_file: PathBuf,
}

fn run_suite_in_new_thread(suite_run_spec: Arc<SuiteRunSpec>) {
    spawn(move || run_suite_in_this_thread(&suite_run_spec).map_err(log_and_return_error));
}

fn run_suite_in_this_thread(suite_run_spec: &SuiteRunSpec) -> Result<()> {
    // We hold the lock as long as `_non_parallel_guard` is in scope
    let _non_parallel_guard = try_acquire_suite_lock(suite_run_spec)?;

    debug!("Running suite {}", &suite_run_spec.suite_name);
    persist_suite_execution_report(
        suite_run_spec,
        &SuiteExecutionReport {
            suite_name: suite_run_spec.suite_name.clone(),
            outcome: ExecutionReport::Executed(produce_suite_results(suite_run_spec)?),
        },
    )
    .context("Reporting suite results failed")?;
    debug!("Suite {} finished", &suite_run_spec.suite_name);

    Ok(())
}

fn try_acquire_suite_lock(suite_run_spec: &SuiteRunSpec) -> Result<MutexGuard<usize>> {
    match suite_run_spec.parallelism_protection.try_lock() {
        Ok(non_parallel_guard) => Ok(non_parallel_guard),
        Err(try_lock_error) => match try_lock_error {
            TryLockError::WouldBlock => {
                bail!(
                    "Failed to acquire lock for suite {}, skipping this run",
                    suite_run_spec.suite_name
                );
            }
            TryLockError::Poisoned(poison_error) => {
                error!(
                    "Lock for suite {} poisoned, unpoisoning",
                    suite_run_spec.suite_name
                );
                Ok(poison_error.into_inner())
            }
        },
    }
}

fn produce_suite_results(suite_run_spec: &SuiteRunSpec) -> Result<AttemptsOutcome> {
    let retry_spec = RetrySpec {
        identifier: Identifier {
            name: &suite_run_spec.suite_name,
            timestamp: Utc::now().format("%Y-%m-%dT%H.%M.%S%.f%z").to_string(),
        },
        working_directory: &suite_run_spec.working_directory,
        n_retries_max: suite_run_spec.suite_config.execution_config.n_retries_max,
        timeout: suite_run_spec.suite_config.execution_config.timeout,
        robot_framework_config: &suite_run_spec.suite_config.robot_framework_config,
    };

    create_dir_all(retry_spec.output_directory()).context(format!(
        "Failed to create directory for suite run: {}",
        retry_spec.output_directory().display()
    ))?;

    let environment = Environment::new(
        &suite_run_spec.suite_name,
        &suite_run_spec.suite_config.environment_config,
    );
    let (attempt_outcomes, output_paths) = run_attempts_until_succesful(
        &Session::new(
            &suite_run_spec.suite_config.session_config,
            &environment,
            &suite_run_spec.termination_flag,
        ),
        retry_spec.attempts(),
    );

    Ok(AttemptsOutcome {
        attempts: attempt_outcomes,
        rebot: if output_paths.is_empty() {
            None
        } else {
            Some(
                Rebot {
                    environment: &environment,
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
    session: &Session,
    attempts: impl Iterator<Item = Attempt<'a>>,
) -> (Vec<AttemptOutcome>, Vec<PathBuf>) {
    let mut outcomes = vec![];
    let mut output_paths: Vec<PathBuf> = vec![];

    for attempt in attempts {
        let (outcome, output_path) = run_attempt(session, &attempt);
        let success = matches!(&outcome, &AttemptOutcome::AllTestsPassed);
        outcomes.push(outcome);
        if let Some(output_path) = output_path {
            output_paths.push(output_path);
        }
        if success {
            break;
        }
    }

    (outcomes, output_paths)
}

fn run_attempt(session: &Session, attempt: &Attempt) -> (AttemptOutcome, Option<PathBuf>) {
    let log_message_start = format!(
        "Suite {}, attempt {}",
        attempt.identifier.name, attempt.index
    );

    match session.run(attempt) {
        Ok(run_outcome) => match run_outcome {
            RunOutcome::TimedOut => {
                error!("{log_message_start}: timed out",);
                (AttemptOutcome::TimedOut, None)
            }
            RunOutcome::Exited(result_code) => match result_code {
                Some(result_code) => match result_code {
                    ResultCode::AllTestsPassed => {
                        debug!("{log_message_start}: all tests passed");
                        (
                            AttemptOutcome::AllTestsPassed,
                            Some(attempt.output_xml_file()),
                        )
                    }
                    ResultCode::EnvironmentFailed => {
                        error!("{log_message_start}: environment failure");
                        (AttemptOutcome::EnvironmentFailure, None)
                    }
                    ResultCode::RobotCommandFailed => {
                        if attempt.output_xml_file().exists() {
                            debug!("{log_message_start}: some tests failed");
                            (
                                AttemptOutcome::TestFailures,
                                Some(attempt.output_xml_file()),
                            )
                        } else {
                            error!("{log_message_start}: Robot Framework failure (no output)");
                            (AttemptOutcome::RobotFrameworkFailure, None)
                        }
                    }
                },
                None => {
                    error!("{log_message_start}: failed to query exit code");
                    (
                        AttemptOutcome::OtherError(
                            "Failed to query exit code of Robot Framework call".into(),
                        ),
                        None,
                    )
                }
            },
        },
        Err(error) => {
            error!("{log_message_start}: {error:?}");
            (AttemptOutcome::OtherError(format!("{error:?}")), None)
        }
    }
}

fn persist_suite_execution_report(
    suite_run_spec: &SuiteRunSpec,
    suite_execution_report: &SuiteExecutionReport,
) -> Result<()> {
    write_file_atomic(
        &to_string(suite_execution_report).context("Serializing suite execution report failed")?,
        &suite_run_spec.working_directory,
        &suite_run_spec.result_file,
    )
}
