use crate::command_spec::CommandSpec;
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
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use tokio_util::sync::CancellationToken;

const RESOURCE_ATTRIBUTES_ENV: &str = "CHECKMK_RESOURCE_ATTRIBUTES";

pub struct PlanRunSpec<'a> {
    pub robot: &'a Robot,
    pub id: &'a str,
    pub environment: &'a Environment,
    pub session: &'a Session,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
    pub output_directory: &'a Utf8Path,
    pub resource_attributes: &'a [(&'static str, String)],
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
        resource_attributes,
        ..
    } = spec;
    let log_message_start = format!("Plan {}, attempt {}", id, attempt.index);
    let mut command_spec = attempt.command_spec;
    set_resource_attributes(&mut command_spec, resource_attributes, attempt.index);

    let run_outcome = match session
        .run(&RunSpec {
            id: &format!("robotmk_plan_{}_attempt_{}", id, attempt.index),
            command_spec: &environment.wrap(command_spec),
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

/// Sets the resource attributes, replacing a variable of the same name configured for the plan:
/// Checkmk joins the traces to the services by them. Windows compares variable names ignoring case.
fn set_resource_attributes(
    command_spec: &mut CommandSpec,
    resource_attributes: &[(&'static str, String)],
    attempt: usize,
) {
    command_spec.envs_rendered_obfuscated.retain(|(key, _)| {
        if cfg!(windows) {
            !key.eq_ignore_ascii_case(RESOURCE_ATTRIBUTES_ENV)
        } else {
            key != RESOURCE_ATTRIBUTES_ENV
        }
    });
    command_spec.add_plain_env(
        RESOURCE_ATTRIBUTES_ENV,
        &render_resource_attributes(resource_attributes, attempt),
    );
}

/// Renders the pairs and the attempt in the format of `OTEL_RESOURCE_ATTRIBUTES`.
fn render_resource_attributes(
    resource_attributes: &[(&'static str, String)],
    attempt: usize,
) -> String {
    resource_attributes
        .iter()
        .map(|(key, value)| format!("{key}={}", percent_encode(value)))
        .chain([format!("cmk.attempt={attempt}")])
        .collect::<Vec<_>>()
        .join(",")
}

/// Characters that are percent-encoded in a value of `OTEL_RESOURCE_ATTRIBUTES`: `,` and `=` as
/// required by the OpenTelemetry specification, `%` as the escape character and space because the
/// decoder strips it around a value.
const ENCODED: &AsciiSet = &CONTROLS.add(b' ').add(b'%').add(b',').add(b'=');

fn percent_encode(value: &str) -> String {
    utf8_percent_encode(value, ENCODED).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_resource_attributes_replaces_configured_variable() {
        let mut command_spec = CommandSpec::new("python");
        command_spec
            .add_obfuscated_env("SECRET", "value")
            .add_obfuscated_env(RESOURCE_ATTRIBUTES_ENV, "cmk.suite=configured");
        set_resource_attributes(&mut command_spec, &[("cmk.plan_run_id", "run".into())], 2);
        assert_eq!(
            command_spec.envs_rendered_obfuscated,
            [("SECRET".into(), "value".into())]
        );
        assert_eq!(
            command_spec.envs_rendered_plain,
            [(
                RESOURCE_ATTRIBUTES_ENV.into(),
                "cmk.plan_run_id=run,cmk.attempt=2".into()
            )]
        );
    }

    #[test]
    fn render_resource_attributes_percent_encodes_values() {
        assert_eq!(
            render_resource_attributes(&[("cmk.suite", "a, b=c 100%".into())], 1),
            "cmk.suite=a%2C%20b%3Dc%20100%25,cmk.attempt=1"
        );
    }

    #[test]
    fn render_resource_attributes_percent_encodes_non_ascii() {
        assert_eq!(
            render_resource_attributes(&[("cmk.suite", "ä\\\";".into())], 1),
            "cmk.suite=%C3%A4\\\";,cmk.attempt=1"
        );
    }
}
