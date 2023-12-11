use crate::environment::{Environment, ResultCode};
use crate::results::{AttemptOutcome, RebotOutcome};
use crate::rf::rebot::Rebot;
use crate::rf::robot::{Attempt, Robot};
use crate::sessions::session::{RunOutcome, RunSpec, Session};
use anyhow::{bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use tokio_util::sync::CancellationToken;

pub fn run_attempts_with_rebot(
    robot: &Robot,
    id: &str,
    environment: &Environment,
    session: &Session,
    timeout: u64,
    cancellation_token: &CancellationToken,
    output_directory: &Utf8Path,
) -> Result<(Vec<AttemptOutcome>, Option<RebotOutcome>)> {
    let mut outcomes = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in robot.attempts(output_directory) {
        let (outcome, output_path) = run_attempt(
            id,
            environment,
            session,
            timeout,
            attempt,
            cancellation_token,
            output_directory,
        )?;
        let success = matches!(&outcome, &AttemptOutcome::AllTestsPassed);
        outcomes.push(outcome);
        if let Some(output_path) = output_path {
            output_paths.push(output_path);
        }
        if success {
            break;
        }
    }

    if output_paths.is_empty() {
        return Ok((outcomes, None));
    }
    let rebot = Rebot {
        environment,
        input_paths: &output_paths,
        path_xml: &output_directory.join("rebot.xml"),
        path_html: &output_directory.join("rebot.html"),
    }
    .rebot();

    Ok((outcomes, Some(rebot)))
}

fn run_attempt(
    id: &str,
    environment: &Environment,
    session: &Session,
    timeout: u64,
    attempt: Attempt,
    cancellation_token: &CancellationToken,
    output_directory: &Utf8Path,
) -> Result<(AttemptOutcome, Option<Utf8PathBuf>)> {
    let log_message_start = format!("Suite {}, attempt {}", id, attempt.index);

    let run_outcome = match session.run(&RunSpec {
        id: &format!("robotmk_suite_{}_attempt_{}", id, attempt.index),
        command_spec: &environment.wrap(attempt.command_spec),
        base_path: &output_directory.join(attempt.index.to_string()),
        timeout,
        cancellation_token,
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
    match environment.create_result_code(exit_code) {
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