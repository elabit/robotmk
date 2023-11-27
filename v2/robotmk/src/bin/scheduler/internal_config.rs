use crate::environment::Environment;
use crate::results::suite_results_directory;
use crate::rf::robot::Robot;
use crate::sessions::session::Session;
use robotmk::{
    config::{Config, WorkingDirectoryCleanupConfig},
    lock::Locker,
    section::Host,
    termination::TerminationFlag,
};

use camino::Utf8PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

pub struct GlobalConfig {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub rcc_binary_path: Utf8PathBuf,
    pub termination_flag: TerminationFlag,
    pub results_directory_locker: Locker,
}

#[derive(Clone)]
pub struct Suite {
    pub id: String,
    pub working_directory: Utf8PathBuf,
    pub results_file: Utf8PathBuf,
    pub execution_interval_seconds: u32,
    pub timeout: u64,
    pub robot: Robot,
    pub environment: Environment,
    pub session: Session,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub termination_flag: TerminationFlag,
    pub parallelism_protection: Arc<Mutex<usize>>,
    pub host: Host,
    pub results_directory_locker: Locker,
}

pub fn from_external_config(
    external_config: Config,
    termination_flag: TerminationFlag,
    results_directory_locker: Locker,
) -> (GlobalConfig, Vec<Suite>) {
    let mut suites: Vec<Suite> = external_config
        .suites
        .into_iter()
        .map(|(suite_id, suite_config)| Suite {
            id: suite_id.clone(),
            working_directory: external_config
                .working_directory
                .join("suites")
                .join(&suite_id),
            results_file: suite_results_directory(&external_config.results_directory)
                .join(format!("{}.json", suite_id)),
            execution_interval_seconds: suite_config.execution_config.execution_interval_seconds,
            timeout: suite_config.execution_config.timeout,
            robot: Robot {
                robot_target: suite_config.robot_framework_config.robot_target,
                command_line_args: suite_config.robot_framework_config.command_line_args,
                n_attempts_max: suite_config.execution_config.n_attempts_max,
                retry_strategy: suite_config.execution_config.retry_strategy,
            },
            environment: Environment::new(
                &suite_id,
                &external_config.rcc_binary_path,
                &suite_config.environment_config,
            ),
            session: Session::new(&suite_config.session_config),
            working_directory_cleanup_config: suite_config.working_directory_cleanup_config,
            termination_flag: termination_flag.clone(),
            parallelism_protection: Arc::new(Mutex::new(0)),
            host: suite_config.host,
            results_directory_locker: results_directory_locker.clone(),
        })
        .collect();
    sort_suites_by_id(&mut suites);
    (
        GlobalConfig {
            working_directory: external_config.working_directory,
            results_directory: external_config.results_directory,
            rcc_binary_path: external_config.rcc_binary_path,
            termination_flag,
            results_directory_locker,
        },
        suites,
    )
}

pub fn sort_suites_by_id(suites: &mut [Suite]) {
    suites.sort_by_key(|suite| suite.id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{RCCEnvironment, SystemEnvironment};
    use crate::sessions::session::{CurrentSession, UserSession};
    use robotmk::config::{
        EnvironmentConfig, ExecutionConfig, RCCEnvironmentConfig, RetryStrategy,
        RobotFrameworkConfig, SessionConfig, SuiteConfig, UserSessionConfig,
    };

    use std::collections::HashMap;
    use std::sync::{atomic::AtomicBool, Arc};

    fn system_suite_config() -> SuiteConfig {
        SuiteConfig {
            robot_framework_config: RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/system/tasks.robot"),
                command_line_args: vec![],
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Incremental,
                execution_interval_seconds: 300,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::System,
            session_config: SessionConfig::Current,
            working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
            host: Host::Source,
        }
    }

    fn rcc_suite_config() -> SuiteConfig {
        SuiteConfig {
            robot_framework_config: RobotFrameworkConfig {
                robot_target: Utf8PathBuf::from("/suite/rcc/tasks.robot"),
                command_line_args: vec![],
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
                execution_interval_seconds: 300,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                robot_yaml_path: Utf8PathBuf::from("/suite/rcc/robot.yaml"),
                build_timeout: 300,
                env_json_path: None,
            }),
            session_config: SessionConfig::SpecificUser(UserSessionConfig {
                user_name: "user".into(),
            }),
            working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(50),
            host: Host::Source,
        }
    }

    #[test]
    fn test_from_external_config() {
        let termination_flag = TerminationFlag::new(Arc::new(AtomicBool::new(false)));
        let (global_config, suites) = from_external_config(
            Config {
                working_directory: Utf8PathBuf::from("/working"),
                results_directory: Utf8PathBuf::from("/results"),
                rcc_binary_path: Utf8PathBuf::from("/bin/rcc"),
                suites: HashMap::from([
                    (String::from("system"), system_suite_config()),
                    (String::from("rcc"), rcc_suite_config()),
                ]),
            },
            termination_flag.clone(),
            Locker::new("/config.json", Some(&termination_flag)),
        );
        assert_eq!(global_config.working_directory, "/working");
        assert_eq!(global_config.results_directory, "/results");
        assert_eq!(global_config.rcc_binary_path, "/bin/rcc");
        assert_eq!(suites.len(), 2);
        assert_eq!(suites[0].id, "rcc");
        assert_eq!(suites[0].working_directory, "/working/suites/rcc");
        assert_eq!(suites[0].results_file, "/results/suites/rcc.json");
        assert_eq!(suites[0].execution_interval_seconds, 300);
        assert_eq!(suites[0].timeout, 60);
        assert_eq!(
            suites[0].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/suite/rcc/tasks.robot"),
                command_line_args: vec![],
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
            }
        );
        assert_eq!(
            suites[0].environment,
            Environment::Rcc(RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/suite/rcc/robot.yaml"),
                controller: "robotmk".into(),
                space: "rcc".into(),
                build_timeout: 300,
                env_json_path: None,
            })
        );
        assert_eq!(
            suites[0].session,
            Session::User(UserSession {
                user_name: "user".into()
            })
        );
        assert_eq!(
            suites[0].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxExecutions(50),
        );
        assert_eq!(suites[1].id, "system");
        assert_eq!(suites[1].working_directory, "/working/suites/system");
        assert_eq!(suites[1].results_file, "/results/suites/system.json");
        assert_eq!(suites[1].execution_interval_seconds, 300);
        assert_eq!(suites[1].timeout, 60);
        assert_eq!(
            suites[1].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/suite/system/tasks.robot"),
                command_line_args: vec![],
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Incremental,
            }
        );
        assert_eq!(
            suites[1].environment,
            Environment::System(SystemEnvironment {})
        );
        assert_eq!(suites[1].session, Session::Current(CurrentSession {}));
        assert_eq!(
            suites[1].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
        );
    }
}
