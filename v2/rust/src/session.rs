use super::attempt::Attempt;
use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor};
use super::config::SessionConfig;
use super::environment::{Environment, ResultCode};
use super::termination::TerminationFlag;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use std::process::Command;

pub enum Session<'a> {
    Current(CurrentSession<'a>),
}

impl Session<'_> {
    pub fn new<'a>(
        session_config: &SessionConfig,
        environment: &'a Environment,
        termination_flag: &'a TerminationFlag,
    ) -> Session<'a> {
        match session_config {
            SessionConfig::Current => Session::<'a>::Current(CurrentSession {
                environment,
                termination_flag,
            }),
            SessionConfig::SpecificUser(_) => panic!("User sessions not yet implemented!"),
        }
    }

    pub fn run(&self, attempt: &Attempt) -> Result<RunOutcome> {
        match self {
            Self::Current(current_session) => current_session.run(attempt),
        }
    }
}

pub struct CurrentSession<'a> {
    environment: &'a Environment,
    termination_flag: &'a TerminationFlag,
}

pub enum RunOutcome {
    Exited(Option<ResultCode>),
    TimedOut,
}

impl CurrentSession<'_> {
    fn run(&self, attempt: &Attempt) -> Result<RunOutcome> {
        match (ChildProcessSupervisor {
            command: self.command_with_configured_stdio(attempt)?,
            timeout: attempt.timeout,
            termination_flag: self.termination_flag,
        }
        .run())?
        {
            ChildProcessOutcome::Exited(exit_status) => match exit_status.code() {
                Some(exit_code) => Ok(RunOutcome::Exited(Some(
                    self.environment.create_result_code(exit_code),
                ))),
                None => Ok(RunOutcome::Exited(None)),
            },
            ChildProcessOutcome::TimedOut => Ok(RunOutcome::TimedOut),
        }
    }

    fn command_with_configured_stdio(&self, attempt: &Attempt) -> Result<Command> {
        let mut command = Command::from(&self.environment.wrap(attempt.command_spec()));
        let stdio_paths = stdio_paths_for_attempt(attempt);
        command
            .stdout(std::fs::File::create(&stdio_paths.stdout).context(format!(
                "Failed to open {} for stdout capturing",
                stdio_paths.stdout
            ))?)
            .stderr(std::fs::File::create(&stdio_paths.stderr).context(format!(
                "Failed to open {} for stderr capturing",
                stdio_paths.stderr
            ))?);
        Ok(command)
    }
}

fn stdio_paths_for_attempt(attempt: &Attempt) -> StdioPaths {
    StdioPaths {
        stdout: attempt
            .output_directory
            .join(format!("{}.stdout", attempt.index)),
        stderr: attempt
            .output_directory
            .join(format!("{}.stderr", attempt.index)),
    }
}

struct StdioPaths {
    stdout: Utf8PathBuf,
    stderr: Utf8PathBuf,
}
