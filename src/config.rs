use crate::section::Host;
use anyhow::Result as AnyhowResult;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::fs::read_to_string;

pub fn load(path: &Utf8Path) -> AnyhowResult<Config> {
    Ok(from_str(&read_to_string(path)?)?)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub suite_groups: SequentialSuiteGroups,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SequentialSuiteGroups {
    EnterpriseMode(EnterpriseSequentialSuiteGroups),
    CoreMode(Vec<CoreSequentialSuiteGroup>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnterpriseSequentialSuiteGroups {
    pub rcc_config: RCCConfig,
    pub suite_groups: Vec<EnterpriseSequentialSuiteGroup>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RCCConfig {
    pub binary_path: Utf8PathBuf,
    pub profile_config: RCCProfileConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum RCCProfileConfig {
    Default,
    Custom(CustomRCCProfileConfig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CustomRCCProfileConfig {
    pub name: String,
    pub path: Utf8PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnterpriseSequentialSuiteGroup {
    pub suites: Vec<EnterpriseSuiteConfig>,
    pub execution_interval: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnterpriseSuiteConfig {
    pub core_config: CoreSuiteConfig,
    pub environment_config: EnvironmentConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CoreSequentialSuiteGroup {
    pub suites: Vec<CoreSuiteConfig>,
    pub execution_interval: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CoreSuiteConfig {
    pub id: String,
    pub robot_config: RobotConfig,
    pub execution_config: ExecutionConfig,
    pub session_config: SessionConfig,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub host: Host,
    pub metadata: SuiteMetadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RobotConfig {
    pub robot_target: Utf8PathBuf,
    pub command_line_args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExecutionConfig {
    pub n_attempts_max: usize,
    pub retry_strategy: RetryStrategy,
    pub timeout: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum RetryStrategy {
    Incremental,
    Complete,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum EnvironmentConfig {
    System,
    Rcc(RCCEnvironmentConfig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RCCEnvironmentConfig {
    pub robot_yaml_path: Utf8PathBuf,
    pub build_timeout: u64,
    pub env_json_path: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SessionConfig {
    Current,
    SpecificUser(UserSessionConfig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UserSessionConfig {
    pub user_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum WorkingDirectoryCleanupConfig {
    MaxAgeSecs(u64),
    MaxExecutions(usize),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SuiteMetadata {
    pub application: String,
    pub variant: String,
}
