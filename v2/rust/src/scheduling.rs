use super::attempt::{Attempt, Identifier, RetrySpec};
use super::config::{Config, SuiteConfig};
use super::environment::{Environment, ResultCode};
use super::logging::log_and_return_error;
use super::results::{suite_result_file, suite_results_directory};
use super::session::{RunOutcome, Session};
use super::termination::TerminationFlag;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clokwerk::{Scheduler, TimeUnits};
use log::{debug, error};
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
    run_attempts_until_succesful(suite_run_spec)?;
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

fn run_attempts_until_succesful(suite_run_spec: &SuiteRunSpec) -> Result<()> {
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
    let session = Session::new(
        &suite_run_spec.suite_config.session_config,
        &environment,
        &suite_run_spec.termination_flag,
    );
    for attempt in retry_spec.attempts() {
        if run_attempt(&session, &attempt) {
            break;
        }
    }

    Ok(())
}

fn run_attempt(session: &Session, attempt: &Attempt) -> bool {
    let log_message_start = format!(
        "Suite {}, attempt {}",
        attempt.identifier.name, attempt.index
    );

    match session.run(attempt) {
        Ok(run_outcome) => match run_outcome {
            RunOutcome::TimedOut => {
                error!("{log_message_start}: timed out",);
                false
            }
            RunOutcome::Exited(result_code) => match result_code {
                Some(result_code) => match result_code {
                    ResultCode::AllTestsPassed => {
                        debug!("{log_message_start}: all tests passed");
                        true
                    }
                    ResultCode::EnvironmentFailed => {
                        error!("{log_message_start}: environment failure");
                        false
                    }
                    ResultCode::RobotCommandFailed => {
                        if attempt.output_xml_file().exists() {
                            debug!("{log_message_start}: some tests failed");
                        } else {
                            error!("{log_message_start}: Robot Framework failure (no output)");
                        }
                        false
                    }
                },
                None => {
                    error!("{log_message_start}: failed to query exit code");
                    false
                }
            },
        },
        Err(error) => {
            error!("{log_message_start}: {error:?}");
            false
        }
    }
}
