use super::command_spec::CommandSpec;
use super::config::external::{RetryStrategy, RobotFrameworkConfig};
use camino::Utf8PathBuf;

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

    pub fn command_spec(&self) -> CommandSpec {
        let mut command_spec = CommandSpec::new(PYTHON_EXECUTABLE);
        command_spec.add_argument("-m").add_argument("robot");
        if let Some(variable_file) = &self.robot_framework_config.variable_file {
            command_spec
                .add_argument("--variablefile")
                .add_argument(variable_file);
        }
        if let Some(argument_file) = &self.robot_framework_config.argument_file {
            command_spec
                .add_argument("--argumentfile")
                .add_argument(argument_file);
        }
        if matches!(
            self.robot_framework_config.retry_strategy,
            RetryStrategy::Incremental
        ) && self.index > 0
        {
            command_spec.add_argument("--rerunfailed").add_argument(
                self.output_directory
                    .join(format!("{}.xml", self.index - 1)),
            );
        };
        command_spec
            .add_argument("--outputdir")
            .add_argument(&self.output_directory)
            .add_argument("--output")
            .add_argument(self.output_xml_file())
            .add_argument("--log")
            .add_argument(self.output_directory.join(format!("{}.html", self.index)))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument(&self.robot_framework_config.robot_target);
        command_spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_first_run() -> CommandSpec {
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--outputdir")
            .add_argument("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00")
            .add_argument("--output")
            .add_argument(
                Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"),
            )
            .add_argument("--log")
            .add_argument(
                Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.html"),
            )
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/suite/calculator.robot");
        expected
    }

    #[test]
    fn create_complete_command_spec() {
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
        let command_spec = attempt.command_spec();
        // Assert
        assert_eq!(command_spec, expected);
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
        let command_spec = attempt.command_spec();
        // Assert
        assert_eq!(command_spec, expected);
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
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--rerunfailed")
            .add_argument(
                Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"),
            )
            .add_argument("--outputdir")
            .add_argument("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00")
            .add_argument("--output")
            .add_argument(
                Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.xml"),
            )
            .add_argument("--log")
            .add_argument(
                Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.html"),
            )
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/suite/calculator.robot");
        // Act
        let command_spec = attempt.command_spec();
        // Assert
        assert_eq!(command_spec, expected)
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
