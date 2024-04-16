use super::robot::PYTHON_EXECUTABLE;
use crate::command_spec::CommandSpec;
use crate::environment::Environment;
use crate::environment::ResultCode;
use crate::results::{RebotOutcome, RebotResult};
use crate::session::{RunSpec, Session};
use crate::termination::{Cancelled, Outcome};

use anyhow::Result as AnyhowResult;
use base64::{engine::general_purpose, Engine};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::error;
use std::fs::{read, read_to_string};
use tokio_util::sync::CancellationToken;

pub struct Rebot<'a> {
    pub plan_id: &'a str,
    pub environment: &'a Environment,
    pub session: &'a Session,
    pub working_directory: &'a Utf8Path,
    pub cancellation_token: &'a CancellationToken,
    pub input_paths: &'a [Utf8PathBuf],
    pub path_xml: &'a Utf8Path,
    pub path_html: &'a Utf8Path,
}

impl Rebot<'_> {
    pub fn rebot(&self) -> Result<RebotOutcome, Cancelled> {
        let timestamp = Utc::now().timestamp();
        let outcome = match self.run() {
            Ok(outcome) => outcome,
            Err(error) => {
                error!("Rebot execution failed: {error:?}");
                return Ok(RebotOutcome::Error(format!(
                    "Rebot execution failed: {error:?}"
                )));
            }
        };
        let exit_code = match outcome {
            Outcome::Completed(exit_code) => exit_code,
            Outcome::Timeout => {
                error!("Rebot run timed out");
                return Ok(RebotOutcome::Error("Timeout".into()));
            }
            Outcome::Cancel => {
                error!("Rebot run was cancelled");
                return Err(Cancelled {});
            }
        };
        match self.environment.create_result_code(exit_code) {
            ResultCode::AllTestsPassed => Ok(self.process_successful_run(timestamp)),
            ResultCode::RobotCommandFailed => {
                if self.path_xml.exists() {
                    Ok(self.process_successful_run(timestamp))
                } else {
                    error!("Rebot run failed (no merged XML found)");
                    Ok(RebotOutcome::Error(
                        "Rebot run failed (no merged XML found), see stdio logs".into(),
                    ))
                }
            }
            ResultCode::EnvironmentFailed => {
                error!("Environment failure when running rebot");
                Ok(RebotOutcome::Error(
                    "Environment failure when running rebot, see stdio logs".into(),
                ))
            }
        }
    }

    fn run(&self) -> AnyhowResult<Outcome<i32>> {
        self.session.run(&RunSpec {
            id: &format!("robotmk_rebot_{}", self.plan_id),
            command_spec: &self.environment.wrap(self.build_rebot_command_spec()),
            base_path: &self.working_directory.join("rebot"),
            timeout: 120,
            cancellation_token: self.cancellation_token,
        })
    }

    fn build_rebot_command_spec(&self) -> CommandSpec {
        let mut rebot_command_spec: CommandSpec = CommandSpec::new(PYTHON_EXECUTABLE);
        rebot_command_spec
            .add_argument("-m")
            .add_argument("robot.rebot")
            .add_argument("--output")
            .add_argument(self.path_xml)
            .add_argument("--log")
            .add_argument(self.path_html)
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("--merge")
            .add_arguments(self.input_paths);
        rebot_command_spec
    }

    fn process_successful_run(&self, timestamp: i64) -> RebotOutcome {
        match read_to_string(self.path_xml) {
            Ok(merged_xml) => match read(self.path_html) {
                Ok(merged_html) => RebotOutcome::Ok(RebotResult {
                    xml: merged_xml,
                    html_base64: general_purpose::STANDARD.encode(merged_html),
                    timestamp,
                }),
                Err(error) => {
                    let error_message = format!(
                        "Failed to read merged HTML file content from {}: {error:?}",
                        self.path_html
                    );
                    error!("{error_message}");
                    RebotOutcome::Error(error_message)
                }
            },
            Err(error) => {
                let error_message = format!(
                    "Failed to read merged XML file content from {}: {error:?}",
                    self.path_xml
                );
                error!("{error_message}");
                RebotOutcome::Error(error_message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnvironmentConfig;
    use crate::session::CurrentSession;

    #[test]
    fn build_rebot_command() {
        let rebot_command_spec = Rebot {
            plan_id: "my_plan",
            environment: &Environment::new(
                "my_plan",
                "/bin/rcc".into(),
                &EnvironmentConfig::System,
            ),
            session: &Session::Current(CurrentSession {}),
            working_directory: &Utf8PathBuf::from("/working/my_plan"),
            cancellation_token: &CancellationToken::default(),
            input_paths: &[
                Utf8PathBuf::from("/working/my_plan/0.xml"),
                Utf8PathBuf::from("/working/my_plan/1.xml"),
            ],
            path_xml: &Utf8PathBuf::from("/working/my_plan/rebot.xml"),
            path_html: &Utf8PathBuf::from("/working/my_plan/rebot.html"),
        }
        .build_rebot_command_spec();
        let mut expected = CommandSpec::new("python");
        expected
            .add_argument("-m")
            .add_argument("robot.rebot")
            .add_argument("--output")
            .add_argument("/working/my_plan/rebot.xml")
            .add_argument("--log")
            .add_argument("/working/my_plan/rebot.html")
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("--merge")
            .add_argument("/working/my_plan/0.xml")
            .add_argument("/working/my_plan/1.xml");
        assert_eq!(rebot_command_spec, expected)
    }
}
