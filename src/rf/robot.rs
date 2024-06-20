use crate::command_spec::CommandSpec;
use crate::config::RetryStrategy;

use camino::{Utf8Path, Utf8PathBuf};

pub const PYTHON_EXECUTABLE: &str = "python";

#[derive(Clone, Debug, PartialEq)]
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
    pub fn new(
        robot_config: RobotConfig,
        n_attempts_max: usize,
        retry_strategy: RetryStrategy,
    ) -> Self {
        Self {
            robot_target: robot_config.robot_target.clone(),
            command_line_args: Self::config_to_command_line_args(robot_config),
            n_attempts_max,
            retry_strategy,
        }
    }

    pub fn attempts<'a>(
        &'a self,
        output_directory: &'a Utf8Path,
    ) -> impl Iterator<Item = Attempt> + 'a {
        (1..(self.n_attempts_max + 1)).map(move |i| self.attempt(output_directory, i))
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
        if matches!(self.retry_strategy, RetryStrategy::Incremental) && index > 1 {
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

    fn config_to_command_line_args(robot_config: RobotConfig) -> Vec<String> {
        let mut args = vec![];
        if let Some(top_level_suite_name) = robot_config.top_level_suite_name {
            args.push("--name".to_string());
            args.push(top_level_suite_name);
        }
        for suite in robot_config.suites {
            args.push("--suite".to_string());
            args.push(suite);
        }
        for test in robot_config.tests {
            args.push("--test".to_string());
            args.push(test);
        }
        for tag in robot_config.test_tags_include {
            args.push("--include".to_string());
            args.push(tag);
        }
        for tag in robot_config.test_tags_exclude {
            args.push("--exclude".to_string());
            args.push(tag);
        }
        for (name, value) in robot_config.variables {
            args.push("--variable".to_string());
            args.push(format!("{}:{}", name, value));
        }
        for file in robot_config.variable_files {
            args.push("--variablefile".to_string());
            args.push(file.to_string());
        }
        for file in robot_config.argument_files {
            args.push("--argumentfile".to_string());
            args.push(file.to_string());
        }
        if robot_config.exit_on_failure {
            args.push("--exitonfailure".to_string());
        }
        args
    }
}

pub struct RobotConfig {
    pub robot_target: Utf8PathBuf,
    pub top_level_suite_name: Option<String>,
    pub suites: Vec<String>,
    pub tests: Vec<String>,
    pub test_tags_include: Vec<String>,
    pub test_tags_exclude: Vec<String>,
    pub variables: Vec<(String, String)>,
    pub variable_files: Vec<Utf8PathBuf>,
    pub argument_files: Vec<Utf8PathBuf>,
    pub exit_on_failure: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_line_args_empty() {
        assert!(Robot::new(
            RobotConfig {
                robot_target: "/suite/tasks.robot".into(),
                top_level_suite_name: None,
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec![],
                argument_files: vec![],
                exit_on_failure: false,
            },
            1,
            RetryStrategy::Incremental
        )
        .command_line_args
        .is_empty(),);
    }

    #[test]
    fn test_command_line_args_non_empty() {
        assert_eq!(
            Robot::new(
                RobotConfig {
                    robot_target: "/suite/tasks.robot".into(),
                    top_level_suite_name: Some("top_suite".into()),
                    suites: vec!["suite1".into(), "suite2".into()],
                    tests: vec!["test1".into(), "test2".into()],
                    test_tags_include: vec!["tag1".into(), "tag2".into()],
                    test_tags_exclude: vec!["tag3".into(), "tag4".into()],
                    variables: vec![("k1".into(), "v1".into()), ("k2".into(), "v2".into())],
                    variable_files: vec![
                        "/suite/varfile1.txt".into(),
                        "/suite/varfile2.txt".into()
                    ],
                    argument_files: vec![
                        "/suite/argfile1.txt".into(),
                        "/suite/argfile2.txt".into()
                    ],
                    exit_on_failure: true,
                },
                1,
                RetryStrategy::Incremental
            )
            .command_line_args,
            vec![
                "--name",
                "top_suite",
                "--suite",
                "suite1",
                "--suite",
                "suite2",
                "--test",
                "test1",
                "--test",
                "test2",
                "--include",
                "tag1",
                "--include",
                "tag2",
                "--exclude",
                "tag3",
                "--exclude",
                "tag4",
                "--variable",
                "k1:v1",
                "--variable",
                "k2:v2",
                "--variablefile",
                "/suite/varfile1.txt",
                "--variablefile",
                "/suite/varfile2.txt",
                "--argumentfile",
                "/suite/argfile1.txt",
                "--argumentfile",
                "/suite/argfile2.txt",
                "--exitonfailure"
            ]
        );
    }

    #[test]
    fn create_complete_command_spec() {
        // Assemble
        let robot = Robot {
            robot_target: "~/calculator_test/calculator.robot".into(),
            n_attempts_max: 1,
            command_line_args: vec![
                "--suite".into(),
                "suite1".into(),
                "--variable".into(),
                "k:v".into(),
            ],
            retry_strategy: RetryStrategy::Complete,
        };
        let output_directory =
            Utf8PathBuf::from("/tmp/calculator_plan/2023-08-29T12.23.44.419347+00.00");
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--suite")
            .add_argument("suite1")
            .add_argument("--variable")
            .add_argument("k:v")
            .add_argument("--outputdir")
            .add_argument(&output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("1.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/calculator_test/calculator.robot");
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("1.xml"), 1);
        // Assert
        assert_eq!(command_spec, expected);
    }

    #[test]
    fn create_incremental_command_first() {
        // Assemble
        let robot = Robot {
            robot_target: "~/calculator_test/calculator.robot".into(),
            n_attempts_max: 2,
            command_line_args: vec![
                "--name".into(),
                "top_suite".into(),
                "--exitonfailure".into(),
            ],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory =
            Utf8PathBuf::from("/tmp/calculator_plan/2023-08-29T12.23.44.419347+00.00");
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--name")
            .add_argument("top_suite")
            .add_argument("--exitonfailure")
            .add_argument("--outputdir")
            .add_argument(&output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("1.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/calculator_test/calculator.robot");
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("1.xml"), 1);
        // Assert
        assert_eq!(command_spec, expected);
    }

    #[test]
    fn create_incremental_command_second() {
        // Assemble
        let robot = Robot {
            robot_target: "~/calculator_test/calculator.robot".into(),
            n_attempts_max: 2,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory =
            Utf8PathBuf::from("/tmp/calculator_plan/2023-08-29T12.23.44.419347+00.00");
        let mut expected = CommandSpec::new(PYTHON_EXECUTABLE);
        expected
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--rerunfailed")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--outputdir")
            .add_argument(&output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("2.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("2.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/calculator_test/calculator.robot");
        // Act
        let command_spec =
            robot.command_spec(&output_directory, &output_directory.join("2.xml"), 2);
        // Assert
        assert_eq!(command_spec, expected)
    }

    #[test]
    fn create_two_attempts() {
        // Assemble
        let robot = Robot {
            robot_target: "~/calculator_test/calculator.robot".into(),
            n_attempts_max: 2,
            command_line_args: vec![],
            retry_strategy: RetryStrategy::Incremental,
        };
        let output_directory =
            Utf8PathBuf::from("/tmp/outputdir/plan_1/2023-08-29T12.23.44.419347+00.00");
        let mut first_command_spec = CommandSpec::new(PYTHON_EXECUTABLE);
        first_command_spec
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--outputdir")
            .add_argument(&output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("1.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/calculator_test/calculator.robot");
        let mut second_command_spec = CommandSpec::new(PYTHON_EXECUTABLE);
        second_command_spec
            .add_argument("-m")
            .add_argument("robot")
            .add_argument("--rerunfailed")
            .add_argument(output_directory.join("1.xml"))
            .add_argument("--outputdir")
            .add_argument(&output_directory)
            .add_argument("--output")
            .add_argument(output_directory.join("2.xml"))
            .add_argument("--log")
            .add_argument(output_directory.join("2.html"))
            .add_argument("--report")
            .add_argument("NONE")
            .add_argument("~/calculator_test/calculator.robot");
        let first_attempt = Attempt {
            index: 1,
            command_spec: first_command_spec,
            output_xml_file: output_directory.join("1.xml"),
        };
        let second_attempt = Attempt {
            index: 2,
            command_spec: second_command_spec,
            output_xml_file: output_directory.join("2.xml"),
        };
        // Act
        let attempts: Vec<Attempt> = robot.attempts(&output_directory).collect();
        // Assert
        assert_eq!(attempts, [first_attempt, second_attempt])
    }
}
