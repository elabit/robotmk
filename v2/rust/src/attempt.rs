use super::config::{RetryStrategy, RobotFrameworkConfig};
use camino::Utf8PathBuf;
use std::process::Command;

pub const PYTHON_EXECUTABLE: &str = "python";

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Identifier<'a> {
    pub name: &'a str,
    pub timestamp: String,
}

pub struct RetrySpec<'a> {
    pub identifier: Identifier<'a>,
    pub working_directory: &'a Utf8PathBuf,
    pub n_retries_max: usize,
    pub timeout: u64,
    pub robot_framework_config: &'a RobotFrameworkConfig,
}

impl RetrySpec<'_> {
    pub fn output_directory(&self) -> Utf8PathBuf {
        self.working_directory
            .join(self.identifier.name)
            .join(&self.identifier.timestamp)
    }

    pub fn attempts(&self) -> impl Iterator<Item = Attempt> + '_ {
        (0..self.n_retries_max).map(|i| Attempt {
            output_directory: self.output_directory(),
            identifier: &self.identifier,
            index: i,
            timeout: self.timeout,
            robot_framework_config: self.robot_framework_config,
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Attempt<'a> {
    pub output_directory: Utf8PathBuf,
    pub identifier: &'a Identifier<'a>,
    pub index: usize,
    pub timeout: u64,
    robot_framework_config: &'a RobotFrameworkConfig,
}

impl Attempt<'_> {
    pub fn output_xml_file(&self) -> Utf8PathBuf {
        self.output_directory.join(format!("{}.xml", self.index))
    }

    pub fn command(&self) -> Command {
        let mut robot_command = Command::new(PYTHON_EXECUTABLE);
        robot_command.arg("-m").arg("robot");
        if let Some(variable_file) = &self.robot_framework_config.variable_file {
            robot_command.arg("--variablefile").arg(variable_file);
        }
        if let Some(argument_file) = &self.robot_framework_config.argument_file {
            robot_command.arg("--argumentfile").arg(argument_file);
        }
        if matches!(
            self.robot_framework_config.retry_strategy,
            RetryStrategy::Incremental
        ) && self.index > 0
        {
            let previous_attempt = self
                .output_directory
                .join(format!("{}.xml", self.index - 1));
            robot_command.arg("--rerunfailed").arg(previous_attempt);
        };
        robot_command.arg("--outputdir").arg(&self.output_directory);
        robot_command.arg("--output").arg(self.output_xml_file());
        robot_command
            .arg("--log")
            .arg(self.output_directory.join(format!("{}.html", self.index)));
        robot_command.arg("--report").arg("NONE");
        robot_command.arg(&self.robot_framework_config.robot_target);
        robot_command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_first_run() -> Command {
        let mut expected = Command::new(PYTHON_EXECUTABLE);
        expected
            .arg("-m")
            .arg("robot")
            .arg("--outputdir")
            .arg("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00")
            .arg("--output")
            .arg(Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"))
            .arg("--log")
            .arg(Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.html"))
            .arg("--report")
            .arg("NONE")
            .arg("~/suite/calculator.robot");
        expected
    }

    #[test]
    fn create_complete_command() {
        // Assemble
        let attempt = Attempt {
            output_directory: "/tmp/my_suite/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: &Identifier {
                name: "my_suite",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            timeout: 200,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Complete,
            },
        };
        let expected = expected_first_run();
        // Act
        let command = attempt.command();
        // Assert
        assert_eq!(format!("{:?}", command), format!("{:?}", expected))
    }

    #[test]
    fn create_incremental_command_first() {
        // Assemble
        let attempt = Attempt {
            output_directory: "/tmp/my_suite/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: &Identifier {
                name: "my_suite",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            timeout: 200,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        };
        let expected = expected_first_run();
        // Act
        let command = attempt.command();
        // Assert
        assert_eq!(format!("{:?}", command), format!("{:?}", expected))
    }

    #[test]
    fn create_incremental_command_second() {
        // Assemble
        let attempt = Attempt {
            output_directory: "/tmp/my_suite/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: &Identifier {
                name: "my_suite",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 1,
            timeout: 200,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        };
        let mut expected = Command::new(PYTHON_EXECUTABLE);
        expected
            .arg("-m")
            .arg("robot")
            .arg("--rerunfailed")
            .arg(Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"))
            .arg("--outputdir")
            .arg("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00")
            .arg("--output")
            .arg(Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.xml"))
            .arg("--log")
            .arg(Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.html"))
            .arg("--report")
            .arg("NONE")
            .arg("~/suite/calculator.robot");
        // Act
        let command = attempt.command();
        // Assert
        assert_eq!(format!("{:?}", command), format!("{:?}", expected))
    }

    #[test]
    fn create_two_attempts() {
        // Assemble
        let spec = RetrySpec {
            identifier: Identifier {
                name: "suite_1",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            working_directory: &Utf8PathBuf::from("/tmp/outputdir/"),
            n_retries_max: 2,
            timeout: 300,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: Some("~/suite/retry.yaml".into()),
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        };
        let first_attempt = Attempt {
            output_directory: "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: &Identifier {
                name: "suite_1",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            timeout: 300,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: Some("~/suite/retry.yaml".into()),
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        };
        let second_attempt = Attempt {
            output_directory: "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: &Identifier {
                name: "suite_1",
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 1,
            timeout: 300,
            robot_framework_config: &RobotFrameworkConfig {
                robot_target: "~/suite/calculator.robot".into(),
                variable_file: Some("~/suite/retry.yaml".into()),
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        };
        // Act
        let attempts: Vec<Attempt> = spec.attempts().collect();
        assert_eq!(attempts, [first_attempt, second_attempt])
    }
}
