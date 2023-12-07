use robotmk::command_spec::CommandSpec;
use robotmk::config::RetryStrategy;

use camino::{Utf8Path, Utf8PathBuf};

pub const PYTHON_EXECUTABLE: &str = "python";

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Robot {
    pub robot_target: Utf8PathBuf,
    pub command_line_args: Vec<String>,
    pub n_attempts_max: usize,
    pub retry_strategy: RetryStrategy,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Attempt {
    pub index: usize,
    pub command_spec: CommandSpec,
    pub output_xml_file: Utf8PathBuf,
}

impl Robot {
    pub fn attempts<'a>(
        &'a self,
        output_directory: &'a Utf8Path,
    ) -> impl Iterator<Item = Attempt> + 'a {
        (0..self.n_attempts_max).map(move |i| self.attempt(output_directory, i))
    }

    fn attempt(&self, output_directory: &Utf8Path, index: usize) -> Attempt {
        let output_xml_file = output_directory.join(format!("{}.xml", index));
        Attempt {
            index,
            command_spec: self.command_spec(output_directory, &output_xml_file, index),
            output_xml_file,
        }
    }

    fn command_spec(
        &self,
        output_directory: &Utf8Path,
        output_xml_file: &Utf8Path,
        index: usize,
    ) -> CommandSpec {
        let mut command_spec = CommandSpec::new(PYTHON_EXECUTABLE);
        command_spec.add_argument("-m").add_argument("robot");
        command_spec.add_arguments(&self.command_line_args);
        if matches!(self.retry_strategy, RetryStrategy::Incremental) && index > 0 {
            command_spec
                .add_argument("--rerunfailed")
                .add_argument(output_directory.join(format!("{}.xml", index - 1)));
        };
        command_spec
            .add_argument("--outputdir")
            .add_argument(output_directory)
            .add_argument("--output")
            .add_argument(output_xml_file)
            .add_argument("--log")
            .add_argument(output_directory.join(format!("{}.html", index)))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument(&self.robot_target);
        command_spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_first_run(output_directory: &Utf8Path) -> CommandSpec {
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--outputdir")
            .add_argument(output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("0.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("0.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/suite/calculator.robot");
        expected
    }

    fn expected_second_run(output_directory: &Utf8Path) -> CommandSpec {
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--rerunfailed")
            .add_argument(output_directory.join("0.xml"))
            .add_argument("--outputdir")
            .add_argument(output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("1.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/suite/calculator.robot");
        expected
    }

    #[test]
    fn create_complete_command_spec() {
        // Assemble
        let robot = Robot {
            robot_target: "~/suite/calculator.robot".into(),
            n_attempts_max: 1,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Complete,
        };
        let output_directory = Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00");
        let expected = expected_first_run(&output_directory);
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("0.xml"), 0);
        // Assert
        assert_eq!(command_spec, expected);
    }

    #[test]
    fn create_incremental_command_first() {
        // Assemble
        let robot = Robot {
            robot_target: "~/suite/calculator.robot".into(),
            n_attempts_max: 1,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory = Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00");
        let expected = expected_first_run(&output_directory);
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("0.xml"), 0);
        // Assert
        assert_eq!(command_spec, expected);
    }

    #[test]
    fn create_incremental_command_second() {
        // Assemble
        let robot = Robot {
            robot_target: "~/suite/calculator.robot".into(),
            n_attempts_max: 1,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory = Utf8PathBuf::from("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00");
        let expected = expected_second_run(&output_directory);
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("1.xml"), 1);
        // Assert
        assert_eq!(command_spec, expected)
    }

    #[test]
    fn create_two_attempts() {
        // Assemble
        let robot = Robot {
            robot_target: "~/suite/calculator.robot".into(),
            n_attempts_max: 2,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory =
            Utf8PathBuf::from("/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00");
        let first_attempt = Attempt {
            index: 0,
            command_spec: expected_first_run(&output_directory),
            output_xml_file: output_directory.join("0.xml"),
        };
        let second_attempt = Attempt {
            index: 1,
            command_spec: expected_second_run(&output_directory),
            output_xml_file: output_directory.join("1.xml"),
        };
        // Act
        let attempts: Vec<Attempt> = robot.attempts(&output_directory).collect();
        // Assert
        assert_eq!(attempts, [first_attempt, second_attempt])
    }
}
