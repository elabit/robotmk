pub mod conda;
pub mod rcc;
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
    CondaFromManifest(conda::CondaEnvironmentFromManifest),
    CondaFromArchive(conda::CondaEnvironmentFromArchive),
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
            Self::CondaFromManifest(conda_environment_from_manifest) => {
                conda_environment_from_manifest.build(
                    id,
                    &Session::Current(CurrentSession {}),
                    start_time,
                    cancellation_token,
                )
            }
            Self::CondaFromArchive(conda_environment_from_archive) => {
                conda_environment_from_archive.build(
                    id,
                    &Session::Current(CurrentSession {}),
                    start_time,
                    cancellation_token,
                )
            }
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        match self {
            Self::System(system_environment) => system_environment.wrap(command_spec),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command_spec),
            Self::CondaFromManifest(conda_environment_from_manifest) => {
                conda_environment_from_manifest.wrap(command_spec)
            }
            Self::CondaFromArchive(conda_environment_from_archive) => {
                conda_environment_from_archive.wrap(command_spec)
            }
        }
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match self {
            Self::System(system_env) => system_env.create_result_code(exit_code),
            Self::Rcc(rcc_env) => rcc_env.create_result_code(exit_code),
            Self::CondaFromManifest(conda_env_from_manifest) => {
                conda_env_from_manifest.create_result_code(exit_code)
            }
            Self::CondaFromArchive(conda_env_from_archive) => {
                conda_env_from_archive.create_result_code(exit_code)
            }
        }
    }
}

pub enum ResultCode {
    Success,
    WrappedCommandFailed,
    EnvironmentFailed,
    Error(String),
}
