use crate::child_process_supervisor::{ChildProcessSupervisor, StdioPaths};
use crate::command_spec::CommandSpec;
use crate::config::SessionConfig;
use crate::tasks::{run_task, TaskSpec};
use crate::termination::Outcome;

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use std::fmt::{Display, Formatter, Result as FmtResult};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
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

    pub fn run(&self, spec: &RunSpec) -> AnyhowResult<Outcome<i32>> {
        match self {
            Self::Current(current_session) => current_session.run(spec),
            Self::User(user_session) => user_session.run(spec),
        }
    }
}

impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Current(current_session) => write!(f, "{}", current_session),
            Self::User(user_session) => write!(f, "{}", user_session),
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct CurrentSession {}

impl Display for CurrentSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Current session")
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct UserSession {
    pub user_name: String,
}

impl Display for UserSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Session of user {}", self.user_name)
    }
}

pub struct RunSpec<'a> {
    pub id: &'a str,
    pub command_spec: &'a CommandSpec,
    pub base_path: &'a Utf8Path,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
}

impl CurrentSession {
    fn run(&self, spec: &RunSpec) -> AnyhowResult<Outcome<i32>> {
        match (ChildProcessSupervisor {
            command_spec: spec.command_spec,
            stdio_paths: Some(StdioPaths {
                stdout: Utf8PathBuf::from(format!("{}.stdout", spec.base_path)),
                stderr: Utf8PathBuf::from(format!("{}.stderr", spec.base_path)),
            }),
            timeout: spec.timeout,
            cancellation_token: spec.cancellation_token,
        }
        .run())?
        {
            Outcome::Completed(exit_status) => Ok(Outcome::Completed(
                exit_status
                    .code()
                    .context("Failed to retrieve exit code of subprocess")?,
            )),
            Outcome::Timeout => Ok(Outcome::Timeout),
            Outcome::Cancel => Ok(Outcome::Cancel),
        }
    }
}

impl UserSession {
    fn run(&self, spec: &RunSpec) -> AnyhowResult<Outcome<i32>> {
        run_task(&TaskSpec {
            task_name: spec.id,
            command_spec: spec.command_spec,
            user_name: &self.user_name,
            base_path: spec.base_path,
            timeout: spec.timeout,
            cancellation_token: spec.cancellation_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_session_fmt() {
        assert_eq!(
            format!(
                "{}",
                UserSession {
                    user_name: "some_user".into()
                }
            ),
            "Session of user some_user"
        )
    }
}
