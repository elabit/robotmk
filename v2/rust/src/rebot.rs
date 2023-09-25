use super::attempt::PYTHON_EXECUTABLE;
use super::environment::Environment;
use super::results::{RebotOutcome, RebotResult};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use log::debug;
use log::error;
use std::fs::{read, read_to_string};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Rebot<'a> {
    pub environment: &'a Environment,
    pub input_paths: &'a [PathBuf],
    pub path_xml: &'a Path,
    pub path_html: &'a Path,
}

impl Rebot<'_> {
    pub fn rebot(&self) -> RebotOutcome {
        match self.run() {
            Ok(output) => {
                if output.status.success() {
                    self.process_successful_run()
                } else {
                    let rebot_run_stdout = String::from_utf8_lossy(&output.stdout);
                    let rebot_run_stderr = String::from_utf8_lossy(&output.stderr);
                    let error_message =
                        format!("Rebot run failed. Stdout:\n{rebot_run_stdout}\n\nStderr:\n{rebot_run_stderr}");
                    error!("{error_message}");
                    RebotOutcome::Error(error_message)
                }
            }
            Err(error) => {
                error!("Calling rebot command failed: {error:?}");
                RebotOutcome::Error(format!("{error:?}"))
            }
        }
    }

    fn run(&self) -> Result<Output> {
        let mut rebot_command = self.environment.wrap(self.build_rebot_command());
        debug!("Calling rebot command: {:?}", rebot_command);
        rebot_command.output().context("Rebot command failed")
    }

    fn build_rebot_command(&self) -> Command {
        let mut rebot_command = Command::new(PYTHON_EXECUTABLE);
        rebot_command
            .arg("-m")
            .arg("robot.rebot")
            .arg("--output")
            .arg(self.path_xml)
            .arg("--log")
            .arg(self.path_html)
            .arg("--report")
            .arg("NONE")
            .args(self.input_paths);
        rebot_command
    }

    fn process_successful_run(&self) -> RebotOutcome {
        match read_to_string(self.path_xml) {
            Ok(merged_xml) => match read(self.path_html) {
                Ok(merged_html) => RebotOutcome::Ok(RebotResult {
                    xml: merged_xml,
                    html_base64: general_purpose::STANDARD.encode(merged_html),
                }),
                Err(error) => {
                    let error_message = format!(
                        "Failed to read merged HTML file content from {}: {error:?}",
                        self.path_html.display()
                    );
                    error!("{error_message}");
                    RebotOutcome::Error(error_message)
                }
            },
            Err(error) => {
                let error_message = format!(
                    "Failed to read merged XML file content from {}: {error:?}",
                    self.path_xml.display()
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

    #[test]
    fn build_rebot_command() {
        let rebot_command = Rebot {
            environment: &Environment::new("my_suite", &EnvironmentConfig::System),
            input_paths: &[
                PathBuf::from("/working/my_suite/0.xml"),
                PathBuf::from("/working/my_suite/1.xml"),
            ],
            path_xml: &PathBuf::from("/working/my_suite/rebot.xml"),
            path_html: &PathBuf::from("/working/my_suite/rebot.html"),
        }
        .build_rebot_command();
        let mut expected = Command::new("python");
        expected
            .arg("-m")
            .arg("robot.rebot")
            .arg("--output")
            .arg("/working/my_suite/rebot.xml")
            .arg("--log")
            .arg("/working/my_suite/rebot.html")
            .arg("--report")
            .arg("NONE")
            .arg("/working/my_suite/0.xml")
            .arg("/working/my_suite/1.xml");
        assert_eq!(format!("{:?}", rebot_command), format!("{:?}", expected))
    }
}
