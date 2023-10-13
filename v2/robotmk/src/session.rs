use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use super::command_spec::CommandSpec;
use super::config::external::SessionConfig;
use super::termination::TerminationFlag;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Session {
    Current(CurrentSession),
}

impl Session {
    pub fn new(session_config: &SessionConfig) -> Session {
        match session_config {
            SessionConfig::Current => Session::Current(CurrentSession {}),
            SessionConfig::SpecificUser(_) => panic!("User sessions not yet implemented!"),
        }
    }

    pub fn run(&self, spec: &RunSpec) -> Result<RunOutcome> {
        match self {
            Self::Current(current_session) => current_session.run(spec),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CurrentSession {}

pub struct RunSpec<'a> {
    pub id: &'a str,
    pub command_spec: &'a CommandSpec,
    pub base_path: &'a Utf8Path,
    pub timeout: u64,
    pub termination_flag: &'a TerminationFlag,
}

pub enum RunOutcome {
    Exited(Option<i32>),
    TimedOut,
}

impl CurrentSession {
    fn run(&self, spec: &RunSpec) -> Result<RunOutcome> {
        match (ChildProcessSupervisor {
            command_spec: spec.command_spec,
            stdio_paths: Some(StdioPaths {
                stdout: Utf8PathBuf::from(format!("{}.stdout", spec.base_path)),
                stderr: Utf8PathBuf::from(format!("{}.stderr", spec.base_path)),
            }),
            timeout: spec.timeout,
            termination_flag: spec.termination_flag,
        }
        .run())?
        {
            ChildProcessOutcome::Exited(exit_status) => match exit_status.code() {
                Some(exit_code) => Ok(RunOutcome::Exited(Some(exit_code))),
                None => Ok(RunOutcome::Exited(None)),
            },
            ChildProcessOutcome::TimedOut => Ok(RunOutcome::TimedOut),
        }
    }
}
