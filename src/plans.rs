use crate::env::{Environment, ResultCode};
use crate::results::{AttemptOutcome, AttemptReport, RebotOutcome};
use crate::rf::rebot::Rebot;
use crate::rf::robot::{Attempt, Robot};
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};
use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use jiff::Timestamp;
use log::{error, info};
use tokio_util::sync::CancellationToken;

pub struct PlanRunSpec<'a> {
    pub robot: &'a Robot,
    pub id: &'a str,
    pub environment: &'a Environment,
    pub session: &'a Session,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
    pub output_directory: &'a Utf8Path,
}

pub fn run_attempts_with_rebot(
    spec: &PlanRunSpec,
) -> Result<(Vec<AttemptReport>, Option<RebotOutcome>), Cancelled> {
    let &PlanRunSpec {
        robot,
        id,
        environment,
        session,
        cancellation_token,
        output_directory,
        ..
    } = spec;
    let mut attempt_reports = vec![];
    let mut output_paths: Vec<Utf8PathBuf> = vec![];

    for attempt in robot.attempts(output_directory) {
        info!("Plan {id}: running attempt {}", attempt.index);
        let attempt_index = attempt.index;
        let starttime = Timestamp::now();
        let (outcome, output_path) = run_attempt(spec, attempt)?;
        let endtime = Timestamp::now();
        let success = matches!(&outcome, &AttemptOutcome::AllTestsPassed);
        attempt_reports.push(AttemptReport {
            index: attempt_index,
            outcome,
            runtime: endtime.duration_since(starttime).as_secs(),
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
    info!("Plan {id}: Running rebot");
    let rebot = Rebot {
        plan_id: id,
        environment,
        session,
        runtime_base_path: output_directory.join("rebot"),
        cancellation_token,
        input_paths: &output_paths,
        path_xml: &output_directory.join("rebot.xml"),
        path_html: &output_directory.join("rebot.html"),
    }
    .rebot()?;

    Ok((attempt_reports, Some(rebot)))
}

fn run_attempt(
    spec: &PlanRunSpec,
    attempt: Attempt,
) -> Result<(AttemptOutcome, Option<Utf8PathBuf>), Cancelled> {
    let &PlanRunSpec {
        id,
        environment,
        session,
        timeout,
        cancellation_token,
        output_directory,
        ..
    } = spec;
    let log_message_start = format!("Plan {}, attempt {}", id, attempt.index);

    let run_outcome = match session
        .run(&RunSpec {
            id: &format!("robotmk_plan_{}_attempt_{}", id, attempt.index),
            command_spec: &environment.wrap(attempt.command_spec),
            runtime_base_path: &output_directory.join(attempt.index.to_string()),
            timeout,
            cancellation_token,
        })
        .context("Plan execution failed")
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
        ResultCode::Success => {
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
        ResultCode::WrappedCommandFailed => {
            if attempt.output_xml_file.exists() {
                info!("{log_message_start}: some tests failed");
                Ok((AttemptOutcome::TestFailures, Some(attempt.output_xml_file)))
            } else {
                error!("{log_message_start}: robot failure (no output)");
                Ok((AttemptOutcome::RobotFailure, None))
            }
        }
        ResultCode::Error(error) => {
            if attempt.output_xml_file.exists() {
                info!("{log_message_start}: some tests failed");
                Ok((AttemptOutcome::TestFailures, Some(attempt.output_xml_file)))
            } else {
                error!("{log_message_start}: {error} (no output)");
                Ok((
                    AttemptOutcome::OtherError(format!(
                        "{error} (no output), see {output_directory} for stdio logs"
                    )),
                    None,
                ))
            }
        }
    }
}
