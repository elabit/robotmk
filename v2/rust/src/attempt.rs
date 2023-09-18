use std::path::PathBuf;
use std::process::Command;

const PYTHON_EXECUTABLE: &str = "python";

#[derive(Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
enum RetryStrategy {
    Incremental,
    Complete,
}

#[derive(Clone)]
struct Variant {
    variable_file: Option<PathBuf>,
    argument_file: Option<PathBuf>,
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Identifier {
    name: String,
    timestamp: String,
}

#[derive(Clone)]
pub struct RetrySpec {
    identifier: Identifier,
    robot_target: PathBuf,
    working_directory: PathBuf,
    variants: Vec<Variant>,
    strategy: RetryStrategy,
}

impl RetrySpec {
    pub fn output_directory(&self) -> PathBuf {
        self.working_directory
            .join(&self.identifier.name)
            .join(&self.identifier.timestamp)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Attempt {
    output_directory: PathBuf,
    identifier: Identifier,
    index: usize,
    robot_target: PathBuf,
    variable_file: Option<PathBuf>,
    argument_file: Option<PathBuf>,
    retry_strategy: RetryStrategy,
}

impl Attempt {
    fn output_xml_file(&self) -> PathBuf {
        self.output_directory.join(format!("{}.xml", self.index))
    }

    fn command(&self) -> Command {
        let mut robot_command = Command::new(PYTHON_EXECUTABLE);
        robot_command.arg("-m").arg("robot");
        if let Some(variable_file) = &self.variable_file {
            robot_command.arg("--variablefile").arg(variable_file);
        }
        if let Some(argument_file) = &self.argument_file {
            robot_command.arg("--argumentfile").arg(argument_file);
        }
        if self.retry_strategy == RetryStrategy::Incremental && self.index > 0 {
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
        robot_command.arg(&self.robot_target);
        robot_command
    }
}

pub fn create_attempts(spec: RetrySpec) -> Vec<Attempt> {
    let mut attempts = vec![];
    let output_directory = spec.output_directory();
    let RetrySpec {
        identifier,
        robot_target,
        working_directory: _,
        variants,
        strategy,
    } = spec;

    for (i, variant) in variants.into_iter().enumerate() {
        attempts.push(Attempt {
            output_directory: output_directory.clone(),
            identifier: identifier.clone(),
            index: i,
            robot_target: robot_target.clone(),
            variable_file: variant.variable_file,
            argument_file: variant.argument_file,
            retry_strategy: strategy.clone(),
        })
    }
    attempts
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
            .arg(PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"))
            .arg("--log")
            .arg(PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.html"))
            .arg("~/suite/calculator.robot");
        expected
    }

    #[test]
    fn create_complete_command() {
        // Assemble
        let attempt = Attempt {
            output_directory: "/tmp/my_suite/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: Identifier {
                name: "my_suite".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            robot_target: "~/suite/calculator.robot".into(),
            variable_file: None,
            argument_file: None,
            retry_strategy: RetryStrategy::Complete,
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
            identifier: Identifier {
                name: "my_suite".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            robot_target: "~/suite/calculator.robot".into(),
            variable_file: None,
            argument_file: None,
            retry_strategy: RetryStrategy::Incremental,
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
            identifier: Identifier {
                name: "my_suite".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 1,
            robot_target: "~/suite/calculator.robot".into(),
            variable_file: None,
            argument_file: None,
            retry_strategy: RetryStrategy::Incremental,
        };
        let mut expected = Command::new(PYTHON_EXECUTABLE);
        expected
            .arg("-m")
            .arg("robot")
            .arg("--rerunfailed")
            .arg(PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("0.xml"))
            .arg("--outputdir")
            .arg("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00")
            .arg("--output")
            .arg(PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.xml"))
            .arg("--log")
            .arg(PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00").join("1.html"))
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
                name: "suite_1".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            robot_target: "~/suite/calculator.robot".into(),
            working_directory: "/tmp/outputdir/".into(),
            variants: vec![
                Variant {
                    variable_file: None,
                    argument_file: None,
                },
                Variant {
                    variable_file: Some("~/suite/retry.yaml".into()),
                    argument_file: None,
                },
            ],
            strategy: RetryStrategy::Incremental,
        };
        let first_attempt = Attempt {
            output_directory: "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: Identifier {
                name: "suite_1".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 0,
            robot_target: "~/suite/calculator.robot".into(),
            variable_file: None,
            argument_file: None,
            retry_strategy: RetryStrategy::Incremental,
        };
        let second_attempt = Attempt {
            output_directory: "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00".into(),
            identifier: Identifier {
                name: "suite_1".into(),
                timestamp: "2023-08-29T12.23.44.419347+00.00".into(),
            },
            index: 1,
            robot_target: "~/suite/calculator.robot".into(),
            variable_file: Some("~/suite/retry.yaml".into()),
            argument_file: None,
            retry_strategy: RetryStrategy::Incremental,
        };
        // Act
        let attempts = create_attempts(spec);
        assert_eq!(attempts, [first_attempt, second_attempt])
    }
}
