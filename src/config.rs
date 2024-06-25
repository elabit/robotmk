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
    pub managed_directory: Utf8PathBuf,
    pub rcc_config: RCCConfig,
    pub plan_groups: Vec<SequentialPlanGroup>,
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
pub struct SequentialPlanGroup {
    pub plans: Vec<PlanConfig>,
    pub execution_interval: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Source {
    Manual { base_dir: Utf8PathBuf },
    Managed { tar_gz_path: Utf8PathBuf },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlanConfig {
    pub id: String,
    pub source: Source,
    pub robot_config: RobotConfig,
    pub execution_config: ExecutionConfig,
    pub environment_config: EnvironmentConfig,
    pub session_config: SessionConfig,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub host: Host,
    pub metadata: PlanMetadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RobotConfig {
    pub robot_target: Utf8PathBuf,
    pub top_level_suite_name: Option<String>,
    pub suites: Vec<String>,
    pub tests: Vec<String>,
    pub test_tags_include: Vec<String>,
    pub test_tags_exclude: Vec<String>,
    pub variables: Vec<(String, String)>,
    pub variable_files: Vec<Utf8PathBuf>,
    pub argument_files: Vec<Utf8PathBuf>,
    pub exit_on_failure: bool,
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
pub struct PlanMetadata {
    pub application: String,
    pub suite_name: String,
    pub variant: String,
}
