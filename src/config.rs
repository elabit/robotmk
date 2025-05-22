use crate::section::Host;
use anyhow::{Result as AnyhowResult, anyhow, bail};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Deserializer, Serialize};
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
    pub micromamba_binary_path: ValidatedMicromambaBinaryPath,
    pub base_directory: Utf8PathBuf,
}

// Micromamba is very particular regarding the filename of its own executable. Only `micromamba` or
// `micromamba.exe` are accepted. If the filename is different, micromamba will complain:
// Error unknown MAMBA_EXE: "/tmp/not-micromamba", filename must be mamba or micromamba
// /tmp/mambaf893b04kxn5: line 3: not-micromamba: command not found
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ValidatedMicromambaBinaryPath(Utf8PathBuf);

impl TryFrom<Utf8PathBuf> for ValidatedMicromambaBinaryPath {
    type Error = anyhow::Error;

    fn try_from(path: Utf8PathBuf) -> Result<Self, Self::Error> {
        Self::from_utf8_path_buf(path)
    }
}

impl From<ValidatedMicromambaBinaryPath> for Utf8PathBuf {
    fn from(value: ValidatedMicromambaBinaryPath) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for ValidatedMicromambaBinaryPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_utf8_path_buf(Utf8PathBuf::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

impl ValidatedMicromambaBinaryPath {
    const EXPECTED_FILE_NAME: &str = {
        #[cfg(unix)]
        {
            "micromamba"
        }
        #[cfg(windows)]
        {
            "micromamba.exe"
        }
    };

    fn from_utf8_path_buf(path: Utf8PathBuf) -> AnyhowResult<Self> {
        let file_name = path.file_name().ok_or(anyhow!(
            "Micromamba binary path must be a file, got: {}",
            path
        ))?;

        if file_name != Self::EXPECTED_FILE_NAME {
            bail!(
                "Micromamba binary path must be a file named '{expected_file_name}', got: {path}",
                expected_file_name = Self::EXPECTED_FILE_NAME,
            );
        }
        Ok(Self(path))
    }
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
    pub build_timeout: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CondaEnvironmentSource {
    Manifest(CondaEnvironmentFromManifest),
    Archive(Utf8PathBuf),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CondaEnvironmentFromManifest {
    pub manifest_path: Utf8PathBuf,
    pub http_proxy_config: HTTPProxyConfig,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HTTPProxyConfig {
    pub http: Option<String>,
    pub https: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn validated_micromamba_binary_path_from_utf8_path_buf_ok() {
        assert_eq!(
            ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("/micromamba"))
                .unwrap()
                .0,
            "/micromamba"
        )
    }

    #[test]
    #[cfg(windows)]
    fn validated_micromamba_binary_path_from_utf8_path_buf_ok() {
        assert_eq!(
            ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("C:\\micromamba.exe"))
                .unwrap()
                .0,
            "C:\\micromamba.exe"
        )
    }

    #[test]
    #[cfg(unix)]
    fn validated_micromamba_binary_path_from_utf8_path_buf_error() {
        assert!(
            ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("/not-micromamba"),).is_err()
        )
    }

    #[test]
    #[cfg(windows)]
    fn validated_micromamba_binary_path_from_utf8_path_buf_error() {
        assert!(
            ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("C:\\not-micromamba.exe"),)
                .is_err()
        )
    }

    #[test]
    #[cfg(unix)]
    fn utf8_path_buf_from_validated_micromamba_binary_path() {
        assert_eq!(
            Utf8PathBuf::from(
                ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("/micromamba")).unwrap()
            ),
            "/micromamba"
        )
    }

    #[test]
    #[cfg(windows)]
    fn utf8_path_buf_from_validated_micromamba_binary_path() {
        assert_eq!(
            Utf8PathBuf::from(
                ValidatedMicromambaBinaryPath::try_from(Utf8PathBuf::from("C:\\micromamba.exe"))
                    .unwrap()
            ),
            "C:\\micromamba.exe"
        )
    }

    #[test]
    #[cfg(unix)]
    fn deserialize_validated_micromamba_binary_path_ok() {
        assert_eq!(
            serde_json::from_str::<ValidatedMicromambaBinaryPath>("\"/micromamba\"")
                .unwrap()
                .0,
            "/micromamba"
        )
    }

    #[test]
    #[cfg(windows)]
    fn deserialize_validated_micromamba_binary_path_ok() {
        assert_eq!(
            serde_json::from_str::<ValidatedMicromambaBinaryPath>("\"C:\\\\micromamba.exe\"")
                .unwrap()
                .0,
            "C:\\micromamba.exe"
        )
    }

    #[test]
    #[cfg(unix)]
    fn deserialize_validated_micromamba_binary_path_error() {
        assert!(
            serde_json::from_str::<ValidatedMicromambaBinaryPath>("\"/not-micromamba\"").is_err()
        )
    }

    #[test]
    #[cfg(windows)]
    fn deserialize_validated_micromamba_binary_path_error() {
        assert!(
            serde_json::from_str::<ValidatedMicromambaBinaryPath>("\"C:\\\\not-micromamba.exe\"")
                .is_err()
        )
    }
}
