use crate::section::Host;
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::collections::HashMap;
use std::fs::read_to_string;

pub fn load(path: &Utf8Path) -> Result<Config> {
    Ok(from_str(&read_to_string(path)?)?)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub rcc_binary_path: Utf8PathBuf,
    pub suites: HashMap<String, SuiteConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SuiteConfig {
    pub robot_framework_config: RobotFrameworkConfig,
    pub execution_config: ExecutionConfig,
    pub environment_config: EnvironmentConfig,
    pub session_config: SessionConfig,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub host: Host,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RobotFrameworkConfig {
    pub robot_target: Utf8PathBuf,
    pub command_line_args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExecutionConfig {
    pub n_attempts_max: usize,
    pub retry_strategy: RetryStrategy,
    pub execution_interval_seconds: u32,
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
