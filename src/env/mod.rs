pub mod conda;
pub mod rcc;
mod robotmk_env_manifest;
pub mod system;

use crate::command_spec::CommandSpec;
use crate::results::BuildOutcome;
use crate::session::{CurrentSession, Session};
use crate::termination::Cancelled;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
    System(system::SystemEnvironment),
    Rcc(rcc::RCCEnvironment),
    Conda(conda::CondaEnvironment),
}

impl Environment {
    pub fn build(
        &self,
        id: &str,
        session: &Session,
        start_time: DateTime<Utc>,
        cancellation_token: &CancellationToken,
    ) -> Result<BuildOutcome, Cancelled> {
        match self {
            Self::System(system_environment) => system_environment.build(),
            Self::Rcc(rcc_environment) => {
                rcc_environment.build(id, session, start_time, cancellation_token)
            }
            Self::Conda(conda_environment) => conda_environment.build(
                id,
                &Session::Current(CurrentSession {}),
                start_time,
                cancellation_token,
            ),
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        match self {
            Self::System(system_environment) => system_environment.wrap(command_spec),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command_spec),
            Self::Conda(conda_environment) => conda_environment.wrap(command_spec),
        }
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match self {
            Self::System(system_env) => system_env.create_result_code(exit_code),
            Self::Rcc(rcc_env) => rcc_env.create_result_code(exit_code),
            Self::Conda(conda_environment) => conda_environment.create_result_code(exit_code),
        }
    }
}

pub enum ResultCode {
    Success,
    WrappedCommandFailed,
    EnvironmentFailed,
    Error(String),
}
