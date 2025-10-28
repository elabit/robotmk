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
    pub runtime_directory: Utf8PathBuf,
    pub rcc_config: RCCConfig,
    pub conda_config: CondaConfig,
    pub plan_groups: Vec<SequentialPlanGroup>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RCCConfig {
    pub binary_path: Utf8PathBuf,
    pub profile_config: RCCProfileConfig,
    pub robocorp_home_base: Utf8PathBuf,
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
pub struct CondaConfig {
    pub micromamba_binary_path: Utf8PathBuf,
    pub base_directory: Utf8PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SequentialPlanGroup {
    pub plans: Vec<PlanConfig>,
    pub execution_interval: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Source {
    Manual {
        base_dir: Utf8PathBuf,
    },
    Managed {
        tar_gz_path: Utf8PathBuf,
        version_number: usize,
        version_label: String,
    },
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
    pub variables: Vec<RobotFrameworkVariable>,
    pub variable_files: Vec<Utf8PathBuf>,
    pub argument_files: Vec<Utf8PathBuf>,
    pub exit_on_failure: bool,
    pub environment_variables_rendered_obfuscated: Vec<RobotFrameworkObfuscatedEnvVar>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RobotFrameworkVariable {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RobotFrameworkObfuscatedEnvVar {
    pub name: String,
    pub value: String,
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
    Conda(CondaEnvironmentConfig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RCCEnvironmentConfig {
    pub robot_yaml_path: Utf8PathBuf,
    pub build_timeout: u64,
    pub remote_origin: Option<String>,
    pub catalog_zip: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CondaEnvironmentConfig {
    pub source: CondaEnvironmentSource,
    pub robotmk_manifest_path: Option<Utf8PathBuf>,
    pub http_proxy_config: HTTPProxyConfig,
    pub tls_certificate_validation: TlsCertificateValidation,
    pub tls_revokation_enabled: bool,
    pub build_timeout: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CondaEnvironmentSource {
    Manifest(Utf8PathBuf),
    Archive(Utf8PathBuf),
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HTTPProxyConfig {
    pub http: Option<String>,
    pub https: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TlsCertificateValidation {
    Enabled,
    Disabled,
    EnabledWithCustomCert(Utf8PathBuf),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SessionConfig {
    Current,
    #[cfg(windows)]
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
