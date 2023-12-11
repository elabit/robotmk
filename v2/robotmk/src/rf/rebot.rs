use super::robot::PYTHON_EXECUTABLE;
use crate::command_spec::CommandSpec;
use crate::environment::Environment;
use crate::environment::ResultCode;
use crate::results::{RebotOutcome, RebotResult};

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::debug;
use log::error;
use std::fs::{read, read_to_string};
use std::process::{Command, Output};

pub struct Rebot<'a> {
    pub environment: &'a Environment,
    pub input_paths: &'a [Utf8PathBuf],
    pub path_xml: &'a Utf8Path,
    pub path_html: &'a Utf8Path,
}

impl Rebot<'_> {
    pub fn rebot(&self) -> RebotOutcome {
        let timestamp = Utc::now().timestamp();
        match self.run() {
            Ok(output) => match output.status.code() {
                Some(exit_code) => match self.environment.create_result_code(exit_code) {
                    ResultCode::AllTestsPassed => self.process_successful_run(timestamp),
                    ResultCode::RobotCommandFailed => {
                        if self.path_xml.exists() {
                            self.process_successful_run(timestamp)
                        } else {
                            Self::process_failure(&output, "Rebot run failed (no merged XML found)")
                        }
                    }
                    ResultCode::EnvironmentFailed => {
                        Self::process_failure(&output, "Environment failure when running rebot")
                    }
                },
                None => Self::process_failure(&output, "Failed to retrieve exit code of rebot run"),
            },
            Err(error) => {
                error!("Calling rebot command failed: {error:?}");
                RebotOutcome::Error(format!("{error:?}"))
            }
        }
    }

    fn run(&self) -> Result<Output> {
        let rebot_command_spec = self.environment.wrap(self.build_rebot_command_spec());
        debug!("Calling rebot command: {rebot_command_spec}");
        Command::from(&rebot_command_spec)
            .output()
            .context("Rebot command failed")
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

    fn process_failure(rebot_command_output: &Output, error_message: &str) -> RebotOutcome {
        let rebot_run_stdout = String::from_utf8_lossy(&rebot_command_output.stdout);
        let rebot_run_stderr = String::from_utf8_lossy(&rebot_command_output.stderr);
        let error_diagnostics =
            format!("{error_message}. Stdout:\n{rebot_run_stdout}\n\nStderr:\n{rebot_run_stderr}");
        error!("{error_diagnostics}");
        RebotOutcome::Error(error_diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnvironmentConfig;

    #[test]
    fn build_rebot_command() {
        let rebot_command_spec = Rebot {
            environment: &Environment::new(
                "my_suite",
                "/bin/rcc".into(),
                &EnvironmentConfig::System,
            ),
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
