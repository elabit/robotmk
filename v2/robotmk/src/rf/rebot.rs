use super::robot::PYTHON_EXECUTABLE;
use crate::command_spec::CommandSpec;
use crate::environment::Environment;
use crate::environment::ResultCode;
use crate::results::{RebotOutcome, RebotResult};
use crate::sessions::session::{RunOutcome, RunSpec, Session};

use anyhow::bail;
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::debug;
use log::error;
use std::fs::{read, read_to_string};
use tokio_util::sync::CancellationToken;

pub struct Rebot<'a> {
    pub suite_id: &'a str,
    pub environment: &'a Environment,
    pub session: &'a Session,
    pub working_directory: &'a Utf8Path,
    pub cancellation_token: &'a CancellationToken,
    pub input_paths: &'a [Utf8PathBuf],
    pub path_xml: &'a Utf8Path,
    pub path_html: &'a Utf8Path,
}

impl Rebot<'_> {
    pub fn rebot(&self) -> RebotOutcome {
        let timestamp = Utc::now().timestamp();
        match self.run() {
            Ok(exit_code) => match self.environment.create_result_code(exit_code) {
                ResultCode::AllTestsPassed => self.process_successful_run(timestamp),
                ResultCode::RobotCommandFailed => {
                    if self.path_xml.exists() {
                        self.process_successful_run(timestamp)
                    } else {
                        error!("Rebot run failed (no merged XML found)");
                        RebotOutcome::Error("Rebot run failed (no merged XML found)".into())
                    }
                }
                ResultCode::EnvironmentFailed => {
                    error!("Environment failure when running rebot");
                    RebotOutcome::Error("Environment failure when running rebot".into())
                }
            },
            Err(error) => {
                error!("Calling rebot command failed: {error:?}");
                RebotOutcome::Error(format!("{error:?}"))
            }
        }
    }

    fn run(&self) -> Result<i32> {
        let rebot_command_spec = self.environment.wrap(self.build_rebot_command_spec());
        debug!("Calling rebot command: {rebot_command_spec}");
        let run_spec = RunSpec {
            id: &format!("robotmk_rebot_{}", self.suite_id),
            command_spec: &rebot_command_spec,
            base_path: &self.working_directory.join("rebot"),
            timeout: 600,
            cancellation_token: self.cancellation_token,
        };
        match self.session.run(&run_spec)? {
            RunOutcome::Exited(exit_code) => {
                if let Some(exit_code) = exit_code {
                    Ok(exit_code)
                } else {
                    bail!("Failed to retrieve exit code of rebot command")
                }
            }
            RunOutcome::TimedOut => {
                bail!("Timed out")
            }
            RunOutcome::Terminated => {
                bail!("Terminated")
            }
        }
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
    use crate::sessions::session::CurrentSession;

    #[test]
    fn build_rebot_command() {
        let rebot_command_spec = Rebot {
            suite_id: "my_suite",
            environment: &Environment::new(
                "my_suite",
                "/bin/rcc".into(),
                &EnvironmentConfig::System,
            ),
            session: &Session::Current(CurrentSession {}),
            working_directory: &Utf8PathBuf::from("/working/my_suite"),
            cancellation_token: &CancellationToken::default(),
            input_paths: &[
                Utf8PathBuf::from("/working/my_suite/0.xml"),
                Utf8PathBuf::from("/working/my_suite/1.xml"),
            ],
            path_xml: &Utf8PathBuf::from("/working/my_suite/rebot.xml"),
            path_html: &Utf8PathBuf::from("/working/my_suite/rebot.html"),
        }
        .build_rebot_command_spec();
        let mut expected = CommandSpec::new("python");
        expected
            .add_argument("-m")
            .add_argument("robot.rebot")
            .add_argument("--output")
            .add_argument("/working/my_suite/rebot.xml")
            .add_argument("--log")
            .add_argument("/working/my_suite/rebot.html")
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("/working/my_suite/0.xml")
            .add_argument("/working/my_suite/1.xml");
        assert_eq!(rebot_command_spec, expected)
    }
}
