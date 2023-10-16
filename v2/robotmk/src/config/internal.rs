use super::external::{Config, ExecutionConfig, RobotFrameworkConfig};
use crate::environment::Environment;
use crate::results::suite_results_directory;
use crate::session::Session;
use crate::termination::TerminationFlag;

use camino::Utf8PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

pub struct GlobalConfig {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub termination_flag: TerminationFlag,
}

#[derive(Clone)]
pub struct Suite {
    pub name: String,
    pub working_directory: Utf8PathBuf,
    pub results_file: Utf8PathBuf,
    pub execution_config: ExecutionConfig,
    pub robot_framework_config: RobotFrameworkConfig,
    pub environment: Environment,
    pub session: Session,
    pub termination_flag: TerminationFlag,
    pub parallelism_protection: Arc<Mutex<usize>>,
}

pub fn from_external_config(
    external_config: Config,
    termination_flag: TerminationFlag,
) -> (GlobalConfig, Vec<Suite>) {
    let mut suites: Vec<Suite> = external_config
        .suites
        .into_iter()
        .map(|(suite_name, suite_config)| Suite {
            name: suite_name.clone(),
            working_directory: external_config.working_directory.join(&suite_name),
            results_file: suite_results_directory(&external_config.results_directory)
                .join(format!("{}.json", suite_name)),
            execution_config: suite_config.execution_config,
            robot_framework_config: suite_config.robot_framework_config,
            environment: Environment::new(&suite_name, &suite_config.environment_config),
            session: Session::new(&suite_config.session_config),
            termination_flag: termination_flag.clone(),
            parallelism_protection: Arc::new(Mutex::new(0)),
        })
        .collect();
    sort_suites_by_name(&mut suites);
    (
        GlobalConfig {
            working_directory: external_config.working_directory.clone(),
            results_directory: external_config.results_directory.clone(),
            termination_flag: termination_flag.clone(),
        },
        suites,
    )
}

fn sort_suites_by_name(suites: &mut [Suite]) {
    suites.sort_by_key(|suite| suite.name.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::external::{
        EnvironmentConfig, RCCEnvironmentConfig, RetryStrategy, SessionConfig, SuiteConfig,
    };
    use crate::environment::{RCCEnvironment, SystemEnvironment};
    use crate::session::CurrentSession;

    use std::collections::HashMap;

    fn system_suite_config() -> SuiteConfig {
        SuiteConfig {
            robot_framework_config: RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/system/tasks.robot"),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
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

    fn rcc_suite_config() -> SuiteConfig {
        SuiteConfig {
            robot_framework_config: RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/rcc/tasks.robot"),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Complete,
            },
            execution_config: ExecutionConfig {
                n_retries_max: 1,
                execution_interval_seconds: 300,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/suite/rcc/robot.yaml"),
                build_timeout: 300,
            }),
            session_config: SessionConfig::Current,
        }
    }

    #[test]
    fn test_from_external_config() {
        let (global_config, suites) = from_external_config(
            Config {
                working_directory: Utf8PathBuf::from("/working"),
                results_directory: Utf8PathBuf::from("/results"),
                suites: HashMap::from([
                    (String::from("system"), system_suite_config()),
                    (String::from("rcc"), rcc_suite_config()),
                ]),
            },
            TerminationFlag::new(),
        );
        assert_eq!(global_config.working_directory, "/working",);
        assert_eq!(global_config.results_directory, "/results",);
        assert_eq!(suites.len(), 2);
        assert_eq!(suites[0].name, "rcc");
        assert_eq!(suites[0].working_directory, "/working/rcc");
        assert_eq!(suites[0].results_file, "/results/suites/rcc.json");
        assert_eq!(
            suites[0].execution_config,
            ExecutionConfig {
                n_retries_max: 1,
                execution_interval_seconds: 300,
                timeout: 60,
            }
        );
        assert_eq!(
            suites[0].robot_framework_config,
            RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/rcc/tasks.robot"),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Complete,
            },
        );
        assert_eq!(
            suites[0].environment,
            Environment::Rcc(RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/suite/rcc/robot.yaml"),
                controller: "robotmk".into(),
                space: "rcc".into(),
                build_timeout: 300,
            })
        );
        assert_eq!(suites[0].session, Session::Current(CurrentSession {}));
        assert_eq!(suites[1].name, "system");
        assert_eq!(suites[1].working_directory, "/working/system");
        assert_eq!(suites[1].results_file, "/results/suites/system.json");
        assert_eq!(
            suites[1].execution_config,
            ExecutionConfig {
                n_retries_max: 1,
                execution_interval_seconds: 300,
                timeout: 60,
            }
        );
        assert_eq!(
            suites[1].robot_framework_config,
            RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/system/tasks.robot"),
                variable_file: None,
                argument_file: None,
                retry_strategy: RetryStrategy::Incremental,
            },
        );
        assert_eq!(
            suites[1].environment,
            Environment::System(SystemEnvironment {})
        );
        assert_eq!(suites[1].session, Session::Current(CurrentSession {}));
    }
}
