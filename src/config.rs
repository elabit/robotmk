use crate::section::Host;
use anyhow::{bail, Error as AnyhowError, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::convert::{AsRef, From, TryFrom};
use std::fs::read_to_string;

pub fn load(path: &Utf8Path) -> AnyhowResult<Config> {
    Ok(from_str(&read_to_string(path)?)?)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    pub working_directory: AbsoluteUtf8Path,
    pub results_directory: AbsoluteUtf8Path,
    pub managed_directory: AbsoluteUtf8Path,
    pub rcc_config: RCCConfig,
    pub plan_groups: Vec<SequentialPlanGroup>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RCCConfig {
    pub binary_path: AbsoluteUtf8Path,
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
    pub path: AbsoluteUtf8Path,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SequentialPlanGroup {
    pub plans: Vec<PlanConfig>,
    pub execution_interval: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Source {
    Manual { base_dir: AbsoluteUtf8Path },
    Managed { zip_file: AbsoluteUtf8Path },
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
    pub robot_target: RelativeUtf8Path,
    pub top_level_suite_name: Option<String>,
    pub suites: Vec<String>,
    pub tests: Vec<String>,
    pub test_tags_include: Vec<String>,
    pub test_tags_exclude: Vec<String>,
    pub variables: Vec<(String, String)>,
    pub variable_files: Vec<RelativeUtf8Path>,
    pub argument_files: Vec<RelativeUtf8Path>,
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
    pub robot_yaml_path: RelativeUtf8Path,
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AbsoluteUtf8Path(Utf8PathBuf);

impl TryFrom<Utf8PathBuf> for AbsoluteUtf8Path {
    type Error = AnyhowError;

    fn try_from(value: Utf8PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            bail!("Path must be absolute")
        }
    }
}

impl From<AbsoluteUtf8Path> for Utf8PathBuf {
    fn from(value: AbsoluteUtf8Path) -> Self {
        value.0
    }
}

impl AsRef<Utf8Path> for AbsoluteUtf8Path {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl<'de> Deserialize<'de> for AbsoluteUtf8Path {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_from(Utf8PathBuf::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelativeUtf8Path(Utf8PathBuf);

impl TryFrom<Utf8PathBuf> for RelativeUtf8Path {
    type Error = AnyhowError;

    fn try_from(value: Utf8PathBuf) -> Result<Self, Self::Error> {
        if value.is_relative() {
            Ok(Self(value))
        } else {
            anyhow::bail!("Path must be relative")
        }
    }
}

impl From<RelativeUtf8Path> for Utf8PathBuf {
    fn from(value: RelativeUtf8Path) -> Self {
        value.0
    }
}

impl AsRef<Utf8Path> for RelativeUtf8Path {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl<'de> Deserialize<'de> for RelativeUtf8Path {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_from(Utf8PathBuf::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[cfg(unix)]
    const ABSOLUTE_PATH: &str = "/1/2/3";
    #[cfg(windows)]
    const ABSOLUTE_PATH: &str = "C:\\1\\2\\3";
    #[cfg(unix)]
    const RELATIVE_PATH: &str = "1/2/3";
    #[cfg(windows)]
    const RELATIVE_PATH: &str = "1\\2\\3";

    fn serialize(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\"))
    }

    #[test]
    fn test_deserialize_absolute_utf8_path_ok() {
        assert_eq!(
            from_str::<AbsoluteUtf8Path>(&serialize(ABSOLUTE_PATH))
                .unwrap()
                .as_ref(),
            ABSOLUTE_PATH
        );
    }

    #[test]
    fn test_deserialize_absolute_utf8_path_error() {
        assert!(from_str::<AbsoluteUtf8Path>(&serialize(RELATIVE_PATH))
            .unwrap_err()
            .to_string()
            .contains("Path must be absolute"));
    }

    #[test]
    fn test_serialize_absolute_utf8_path() {
        assert_eq!(
            to_string(&AbsoluteUtf8Path::try_from(Utf8PathBuf::from(ABSOLUTE_PATH)).unwrap())
                .unwrap(),
            serialize(ABSOLUTE_PATH)
        )
    }

    #[test]
    fn test_deserialize_relative_utf8_path_ok() {
        assert_eq!(
            from_str::<RelativeUtf8Path>(&serialize(RELATIVE_PATH))
                .unwrap()
                .as_ref(),
            RELATIVE_PATH
        );
    }

    #[test]
    fn test_deserialize_relative_utf8_path_error() {
        assert!(from_str::<RelativeUtf8Path>(&serialize(ABSOLUTE_PATH))
            .unwrap_err()
            .to_string()
            .contains("Path must be relative"));
    }

    #[test]
    fn test_serialize_relative_utf8_path() {
        assert_eq!(
            to_string(&RelativeUtf8Path::try_from(Utf8PathBuf::from(RELATIVE_PATH)).unwrap())
                .unwrap(),
            serialize(RELATIVE_PATH)
        )
    }
}
