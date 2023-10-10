use super::child_process_supervisor::{ChildProcessOutcome, ChildProcessSupervisor, StdioPaths};
use super::command_spec::CommandSpec;
use super::config::external::SessionConfig;
use super::schtasks::{run_task, TaskSpec};
use super::termination::TerminationFlag;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Session {
    Current(CurrentSession),
    User(UserSession),
}

impl Session {
    pub fn new(session_config: &SessionConfig) -> Session {
        match session_config {
            SessionConfig::Current => Session::Current(CurrentSession {}),
            SessionConfig::SpecificUser(user_session_config) => Session::User(UserSession {
                user_name: user_session_config.user_name.clone(),
            }),
        }
    }

    pub fn run(&self, spec: &RunSpec) -> Result<RunOutcome> {
        match self {
            Self::Current(current_session) => current_session.run(spec),
            Self::User(user_session) => user_session.run(spec),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CurrentSession {}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct UserSession {
    pub user_name: String,
}

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
    Terminated,
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
            ChildProcessOutcome::Terminated => Ok(RunOutcome::Terminated),
        }
    }
}

impl UserSession {
    fn run(&self, spec: &RunSpec) -> Result<RunOutcome> {
        run_task(&TaskSpec {
            task_name: spec.id,
            command_spec: spec.command_spec,
            user_name: &self.user_name,
            base_path: spec.base_path,
            timeout: spec.timeout,
            termination_flag: spec.termination_flag,
        })
    }
}
