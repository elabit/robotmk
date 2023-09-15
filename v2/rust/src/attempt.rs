use std::path::PathBuf;
use std::process::Command;

const PYTHON_EXECUTABLE: &str = "python";

#[derive(Clone, PartialEq)]
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
