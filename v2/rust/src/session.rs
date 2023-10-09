use super::attempt::Attempt;
use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use super::config::SessionConfig;
use super::environment::{Environment, ResultCode};
use super::termination::TerminationFlag;
use anyhow::Result;

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
            command_spec: self.environment.wrap(attempt.command_spec()),
            stdio_paths: Some(StdioPaths {
                stdout: attempt
                    .output_directory
                    .join(format!("{}.stdout", attempt.index)),
                stderr: attempt
                    .output_directory
                    .join(format!("{}.stderr", attempt.index)),
            }),
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
}
