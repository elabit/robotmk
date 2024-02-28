use robotmk::config::{Config, RCCConfig, SuiteMetadata, WorkingDirectoryCleanupConfig};
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::suite_results_directory;
use robotmk::rf::robot::Robot;
use robotmk::section::Host;
use robotmk::session::Session;

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
    pub timeout: u64,
    pub robot: Robot,
    pub environment: Environment,
    pub session: Session,
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub cancellation_token: CancellationToken,
    pub host: Host,
    pub results_directory_locker: Locker,
    pub metadata: SuiteMetadata,
    pub group_affiliation: GroupAffiliation,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GroupAffiliation {
    pub group_index: usize,
    pub position_in_group: usize,
    pub execution_interval: u64,
}

pub fn from_external_config(
    external_config: Config,
    cancellation_token: CancellationToken,
    results_directory_locker: Locker,
) -> (GlobalConfig, Vec<Suite>) {
    let mut suites = vec![];

    for (group_index, sequential_group) in external_config.suite_groups.into_iter().enumerate() {
        for (suite_index, suite_config) in sequential_group.suites.into_iter().enumerate() {
            suites.push(Suite {
                id: suite_config.id.clone(),
                working_directory: external_config
                    .working_directory
                    .join("suites")
                    .join(&suite_config.id),
                results_file: suite_results_directory(&external_config.results_directory)
                    .join(format!("{}.json", suite_config.id)),
                timeout: suite_config.execution_config.timeout,
                robot: Robot {
                    robot_target: suite_config.robot_config.robot_target,
                    command_line_args: suite_config.robot_config.command_line_args,
                    n_attempts_max: suite_config.execution_config.n_attempts_max,
                    retry_strategy: suite_config.execution_config.retry_strategy,
                },
                environment: Environment::new(
                    &suite_config.id,
                    &external_config.rcc_config.binary_path,
                    &suite_config.environment_config,
                ),
                session: Session::new(&suite_config.session_config),
                working_directory_cleanup_config: suite_config.working_directory_cleanup_config,
                cancellation_token: cancellation_token.clone(),
                host: suite_config.host,
                results_directory_locker: results_directory_locker.clone(),
                metadata: suite_config.metadata,
                group_affiliation: GroupAffiliation {
                    group_index,
                    position_in_group: suite_index,
                    execution_interval: sequential_group.execution_interval,
                },
            });
        }
    }
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

pub fn sort_suites_by_grouping(suites: &mut [Suite]) {
    suites.sort_by_key(|suite| {
        (
            suite.group_affiliation.group_index,
            suite.group_affiliation.position_in_group,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use robotmk::config::{
        CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, RCCEnvironmentConfig,
        RCCProfileConfig, RetryStrategy, RobotConfig, SequentialSuiteGroup, SessionConfig,
        SuiteConfig, UserSessionConfig,
    };
    use robotmk::environment::{Environment, RCCEnvironment, SystemEnvironment};

    fn system_suite_config() -> SuiteConfig {
        SuiteConfig {
            id: "system_tasks.robot_variant".into(),
            robot_config: RobotConfig {
                robot_target: Utf8PathBuf::from("/suite/system/tasks.robot"),
                command_line_args: vec![],
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Incremental,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::System,
            session_config: SessionConfig::Current,
            working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
            host: Host::Source,
            metadata: SuiteMetadata {
                application: "system".into(),
                suite_name: "tasks.robot".into(),
                variant: "variant".into(),
            },
        }
    }

    fn rcc_suite_config() -> SuiteConfig {
        SuiteConfig {
            id: "rcc_tasks.robot_variant".into(),
            robot_config: RobotConfig {
                robot_target: Utf8PathBuf::from("/suite/rcc/tasks.robot"),
                command_line_args: vec![],
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
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
            metadata: SuiteMetadata {
                application: "rcc".into(),
                suite_name: "tasks.robot".into(),
                variant: "variant".into(),
            },
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
                suite_groups: vec![
                    SequentialSuiteGroup {
                        suites: vec![rcc_suite_config()],
                        execution_interval: 300,
                    },
                    SequentialSuiteGroup {
                        suites: vec![system_suite_config()],
                        execution_interval: 300,
                    },
                ],
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
            suites[0].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxExecutions(50),
        );
        assert_eq!(
            suites[0].metadata,
            SuiteMetadata {
                application: "rcc".into(),
                suite_name: "tasks.robot".into(),
                variant: "variant".into(),
            },
        );
        assert_eq!(
            suites[0].group_affiliation,
            GroupAffiliation {
                group_index: 0,
                position_in_group: 0,
                execution_interval: 300,
            }
        );
        assert_eq!(suites[1].id, "system");
        assert_eq!(suites[1].working_directory, "/working/suites/system");
        assert_eq!(suites[1].results_file, "/results/suites/system.json");
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
        assert_eq!(
            suites[1].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
        );
        assert_eq!(
            suites[1].metadata,
            SuiteMetadata {
                application: "system".into(),
                suite_name: "tasks.robot".into(),
                variant: "variant".into(),
            },
        );
        assert_eq!(
            suites[1].group_affiliation,
            GroupAffiliation {
                group_index: 1,
                position_in_group: 0,
                execution_interval: 300,
            }
        );
    }
}
