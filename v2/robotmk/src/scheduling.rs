use super::attempt::{Attempt, Identifier, RetrySpec};
use super::config::{Config, SuiteConfig};
use super::environment::{Environment, ResultCode};
use super::logging::log_and_return_error;
use super::rebot::Rebot;
use super::results::{
    suite_result_file, suite_results_directory, write_file_atomic, AttemptOutcome, AttemptsOutcome,
    ExecutionReport, SuiteExecutionReport,
};
use super::session::{RunOutcome, RunSpec, Session};
use super::termination::TerminationFlag;

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use chrono::Utc;
use clokwerk::{Scheduler, TimeUnits};
use log::{debug, error};
use serde_json::to_string;
use std::fs::create_dir_all;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread::{sleep, spawn};
use std::time::Duration;

pub fn run_suites(config: &Config, termination_flag: &TerminationFlag) -> Result<()> {
    let mut scheduler = Scheduler::new();
    let mut suite_run_specs = vec![];

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
        suite_run_specs.push(suite_run_spec.clone());
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
            error!("Received termination signal while scheduling, waiting for suites to terminate");
            wait_until_all_suites_have_terminated(suite_run_specs);
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
    working_directory: Utf8PathBuf,
    result_file: Utf8PathBuf,
}

fn run_suite_in_new_thread(suite_run_spec: Arc<SuiteRunSpec>) {
    spawn(move || run_suite_in_this_thread(&suite_run_spec).map_err(log_and_return_error));
}

fn run_suite_in_this_thread(suite_run_spec: &SuiteRunSpec) -> Result<()> {
    // We hold the lock as long as `_non_parallel_guard` is in scope
    let _non_parallel_guard = try_acquire_suite_lock(suite_run_spec).map_err(|err| {
        persist_suite_execution_report(
            suite_run_spec,
            &SuiteExecutionReport {
                suite_name: suite_run_spec.suite_name.clone(),
                outcome: ExecutionReport::AlreadyRunning,
            },
        )
        .context("Reporting failure to acquire suite lock failed")
        .err()
        .unwrap_or(err)
    })?;

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
        retry_spec.output_directory()
    ))?;

    let environment = Environment::new(
        &suite_run_spec.suite_name,
        &suite_run_spec.suite_config.environment_config,
    );
    let (attempt_outcomes, output_paths) = run_attempts_until_succesful(
        retry_spec.attempts(),
        &environment,
        &Session::new(&suite_run_spec.suite_config.session_config),
        &suite_run_spec.termination_flag,
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
    attempts: impl Iterator<Item = Attempt<'a>>,
    environment: &Environment,
    session: &Session,
    termination_flag: &TerminationFlag,
) -> (Vec<AttemptOutcome>, Vec<Utf8PathBuf>) {
    let mut outcomes = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in attempts {
        let (outcome, output_path) = run_attempt(&attempt, environment, session, termination_flag);
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

fn run_attempt(
    attempt: &Attempt,
    environment: &Environment,
    session: &Session,
    termination_flag: &TerminationFlag,
) -> (AttemptOutcome, Option<Utf8PathBuf>) {
    let log_message_start = format!(
        "Suite {}, attempt {}",
        attempt.identifier.name, attempt.index
    );

    let run_outcome = match session.run(&RunSpec {
        id: &format!(
            "robotmk_suite_{}_attempt_{}",
            attempt.identifier.name, attempt.index,
        ),
        command_spec: &environment.wrap(attempt.command_spec()),
        base_path: &attempt.output_directory.join(attempt.index.to_string()),
        timeout: attempt.timeout,
        termination_flag,
    }) {
        Ok(run_outcome) => run_outcome,
        Err(error_) => {
            error!("{log_message_start}: {error_:?}");
            return (AttemptOutcome::OtherError(format!("{error_:?}")), None);
        }
    };
    let exit_code = match run_outcome {
        RunOutcome::Exited(exit_code) => exit_code,
        RunOutcome::TimedOut => {
            error!("{log_message_start}: timed out",);
            return (AttemptOutcome::TimedOut, None);
        }
    };
    let exit_code = match exit_code {
        Some(exit_code) => exit_code,
        None => {
            error!("{log_message_start}: failed to query exit code");
            return (
                AttemptOutcome::OtherError(
                    "Failed to query exit code of Robot Framework call".into(),
                ),
                None,
            );
        }
    };
    match environment.create_result_code(exit_code) {
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

fn wait_until_all_suites_have_terminated(suite_run_specs: Vec<Arc<SuiteRunSpec>>) {
    let mut still_running_suite_specs = suite_run_specs;
    while !still_running_suite_specs.is_empty() {
        let mut still_running = vec![];
        for suite_run_spec in still_running_suite_specs {
            let _ = try_acquire_suite_lock(&suite_run_spec).map_err(|_| {
                error!("Suite {} is still running", suite_run_spec.suite_name);
                still_running.push(suite_run_spec.clone())
            });
        }
        still_running_suite_specs = still_running;
        sleep(Duration::from_millis(250));
    }
}
