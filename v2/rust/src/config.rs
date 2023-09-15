use serde::Deserialize;
use serde_json::from_str;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub fn load(path: &Path) -> anyhow::Result<Config> {
    Ok(from_str(&read_to_string(path)?)?)
}

#[derive(Deserialize)]
pub struct Config {
    pub working_directory: PathBuf,
    pub results_directory: PathBuf,
    suites: HashMap<String, SuiteConfig>,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct SuiteConfig {
    pub robot_framework_config: RobotFrameworkConfig,
    pub execution_config: ExecutionConfig,
    pub environment_config: EnvironmentConfig,
    pub session_config: SessionConfig,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct RobotFrameworkConfig {
    pub robot_target: PathBuf,
    pub variable_file: Option<PathBuf>,
    pub argument_file: Option<PathBuf>,
    pub retry_strategy: RetryStrategy,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum RetryStrategy {
    Incremental,
    Complete,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct ExecutionConfig {
    pub n_retries_max: usize,
    pub execution_interval_seconds: u64,
    pub timeout: u64,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum EnvironmentConfig {
    System,
    Rcc(RCCEnvironmentConfig),
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct RCCEnvironmentConfig {
    pub binary_path: PathBuf,
    pub robocorp_home_path: PathBuf,
    pub robot_yaml_path: PathBuf,
    pub build_timeout: u64,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum SessionConfig {
    Current,
    SpecificUser(UserSessionConfig),
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct UserSessionConfig {
    pub user_name: String,
}

impl Config {
    /// Return suites sorted by suite name
    ///
    /// This makes environment reproducible, provided that you start with the same configuration.
    pub fn suites(&self) -> Vec<(&String, &SuiteConfig)> {
        let mut suites: Vec<(&String, &SuiteConfig)> = self.suites.iter().collect();
        suites.sort_by_key(|(suite_name, _suite_config)| suite_name.to_string());
        suites
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_suite_config(suite_name: &str) -> SuiteConfig {
        SuiteConfig {
            robot_framework_config: RobotFrameworkConfig {
                robot_target: PathBuf::from(format!("/suite/{}/tasks.robot", suite_name)),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Complete,
            },
            execution_config: ExecutionConfig {
                n_retries_max: 1,
                execution_interval_seconds: 300,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::System,
            session_config: SessionConfig::Current,
        }
    }

    fn create_config() -> Config {
        Config {
            working_directory: PathBuf::from("/working"),
            results_directory: PathBuf::from("/results"),
            suites: HashMap::from([
                (String::from("suite_b"), create_suite_config("b")),
                (String::from("suite_a"), create_suite_config("a")),
            ]),
        }
    }

    #[test]
    fn suites_sorted() {
        assert_eq!(
            create_config().suites(),
            [
                (&String::from("suite_a"), &create_suite_config("a")),
                (&String::from("suite_b"), &create_suite_config("b"))
            ]
        );
    }
}
