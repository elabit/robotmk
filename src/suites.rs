use crate::environment::{Environment, ResultCode};
use crate::results::{AttemptOutcome, AttemptReport, RebotOutcome};
use crate::rf::rebot::Rebot;
use crate::rf::robot::{Attempt, Robot};
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};
use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::{error, info};
use tokio_util::sync::CancellationToken;

pub fn run_attempts_with_rebot(
    robot: &Robot,
    id: &str,
    environment: &Environment,
    session: &Session,
    timeout: u64,
    cancellation_token: &CancellationToken,
    output_directory: &Utf8Path,
) -> Result<(Vec<AttemptReport>, Option<RebotOutcome>), Cancelled> {
    let mut attempt_reports = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in robot.attempts(output_directory) {
        info!("Suite {id}: running attempt {}", attempt.index);
        let attempt_index = attempt.index;
        let starttime = Utc::now();
        let (outcome, output_path) = run_attempt(
            id,
            environment,
            session,
            timeout,
            attempt,
            cancellation_token,
            output_directory,
        )?;
        let endtime = Utc::now();
        let success = matches!(&outcome, &AttemptOutcome::AllTestsPassed);
        attempt_reports.push(AttemptReport {
            index: attempt_index,
            outcome,
            runtime: (endtime - starttime).num_seconds(),
        });
        if let Some(output_path) = output_path {
            output_paths.push(output_path);
        }
        if success {
            break;
        }
    }

    if output_paths.is_empty() {
        return Ok((attempt_reports, None));
    }
    info!("Suite {id}: Running rebot");
    let rebot = Rebot {
        rmk_id: id,
        environment,
        session,
        working_directory: output_directory,
        cancellation_token,
        input_paths: &output_paths,
        path_xml: &output_directory.join("rebot.xml"),
        path_html: &output_directory.join("rebot.html"),
    }
    .rebot()?;

    Ok((attempt_reports, Some(rebot)))
}

fn run_attempt(
    id: &str,
    environment: &Environment,
    session: &Session,
    timeout: u64,
    attempt: Attempt,
    cancellation_token: &CancellationToken,
    output_directory: &Utf8Path,
) -> Result<(AttemptOutcome, Option<Utf8PathBuf>), Cancelled> {
    let log_message_start = format!("Suite {}, attempt {}", id, attempt.index);

    let run_outcome = match session
        .run(&RunSpec {
            id: &format!("robotmk_suite_{}_attempt_{}", id, attempt.index),
            command_spec: &environment.wrap(attempt.command_spec),
            base_path: &output_directory.join(attempt.index.to_string()),
            timeout,
            cancellation_token,
        })
        .context("Suite execution failed")
    {
        Ok(run_outcome) => run_outcome,
        Err(error_) => {
            error!("{log_message_start}: {error_:?}");
            return Ok((AttemptOutcome::OtherError(format!("{error_:?}")), None));
        }
    };
    let exit_code = match run_outcome {
        Outcome::Completed(exit_code) => exit_code,
        Outcome::Timeout => {
            error!("{log_message_start}: robot run timed out");
            return Ok((AttemptOutcome::TimedOut, None));
        }
        Outcome::Cancel => {
            error!("{log_message_start}: robot run was cancelled");
            return Err(Cancelled {});
        }
    };
    match environment.create_result_code(exit_code) {
        ResultCode::AllTestsPassed => {
            info!("{log_message_start}: all tests passed");
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
                info!("{log_message_start}: some tests failed");
                Ok((AttemptOutcome::TestFailures, Some(attempt.output_xml_file)))
            } else {
                error!("{log_message_start}: robot failure (no output)");
                Ok((AttemptOutcome::RobotFailure, None))
            }
        }
    }
}
