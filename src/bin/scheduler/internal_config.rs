use robotmk::config::{Config, RCCConfig, WorkingDirectoryCleanupConfig};
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::suite_results_directory;
use robotmk::rf::robot::Robot;
use robotmk::section::Host;
use robotmk::sessions::session::Session;

use camino::Utf8PathBuf;
use tokio_util::sync::CancellationToken;

pub struct GlobalConfig {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub rcc_config: RCCConfig,
    pub cancellation_token: CancellationToken,
    pub results_directory_locker: Locker,
}

#[derive(Clone)]
pub struct Suite {
    pub id: String,
    pub working_directory: Utf8PathBuf,
    pub results_file: Utf8PathBuf,
    pub execution_interval_seconds: u64,
    pub timeout: u64,
    pub robot: Robot,
    pub environment: Environment,
    pub session: Session,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub cancellation_token: CancellationToken,
    pub host: Host,
    pub results_directory_locker: Locker,
}

pub fn from_external_config(
    external_config: Config,
    cancellation_token: CancellationToken,
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
                robot_target: suite_config.robot_config.robot_target,
                command_line_args: suite_config.robot_config.command_line_args,
                n_attempts_max: suite_config.execution_config.n_attempts_max,
                retry_strategy: suite_config.execution_config.retry_strategy,
            },
            environment: Environment::new(
                &suite_id,
                &external_config.rcc_config.binary_path,
                &suite_config.environment_config,
            ),
            session: Session::new(&suite_config.session_config),
            working_directory_cleanup_config: suite_config.working_directory_cleanup_config,
            cancellation_token: cancellation_token.clone(),
            host: suite_config.host,
            results_directory_locker: results_directory_locker.clone(),
        })
        .collect();
    sort_suites_by_id(&mut suites);
    (
        GlobalConfig {
            working_directory: external_config.working_directory,
            results_directory: external_config.results_directory,
            rcc_config: external_config.rcc_config,
            cancellation_token,
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
    use robotmk::config::{
        CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, RCCEnvironmentConfig,
        RCCProfileConfig, RetryStrategy, RobotConfig, SessionConfig, SuiteConfig,
        UserSessionConfig,
    };
    use robotmk::environment::{Environment, RCCEnvironment, SystemEnvironment};
    use robotmk::sessions::session::{CurrentSession, UserSession};

    use std::collections::HashMap;

    fn system_suite_config() -> SuiteConfig {
        SuiteConfig {
            robot_config: RobotConfig {
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
            robot_config: RobotConfig {
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
        let cancellation_token = CancellationToken::new();
        let (global_config, suites) = from_external_config(
            Config {
                working_directory: Utf8PathBuf::from("/working"),
                results_directory: Utf8PathBuf::from("/results"),
                rcc_config: RCCConfig {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    profile_config: RCCProfileConfig::Custom(CustomRCCProfileConfig {
                        name: "Robotmk".into(),
                        path: "/rcc_profile_robotmk.yaml".into(),
                    }),
                },
                suites: HashMap::from([
                    (String::from("system"), system_suite_config()),
                    (String::from("rcc"), rcc_suite_config()),
                ]),
            },
            cancellation_token.clone(),
            Locker::new("/config.json", Some(&cancellation_token)),
        );
        assert_eq!(global_config.working_directory, "/working");
        assert_eq!(global_config.results_directory, "/results");
        assert_eq!(
            global_config.rcc_config,
            RCCConfig {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                profile_config: RCCProfileConfig::Custom(CustomRCCProfileConfig {
                    name: "Robotmk".into(),
                    path: "/rcc_profile_robotmk.yaml".into(),
                }),
            }
        );
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