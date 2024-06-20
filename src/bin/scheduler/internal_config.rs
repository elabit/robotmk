use robotmk::config as external_config;
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::plan_results_directory;
use robotmk::rf::robot::{Robot, RobotConfig};
use robotmk::section::Host;
use robotmk::session::Session;

use camino::Utf8PathBuf;
use tokio_util::sync::CancellationToken;

pub struct GlobalConfig {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub managed_directory: Utf8PathBuf,
    pub rcc_config: RCCConfig,
    pub cancellation_token: CancellationToken,
    pub results_directory_locker: Locker,
}

#[derive(PartialEq, Debug)]
pub struct RCCConfig {
    pub binary_path: Utf8PathBuf,
    pub profile_config: RCCProfileConfig,
}

#[derive(PartialEq, Debug)]
pub enum RCCProfileConfig {
    Default,
    Custom(CustomRCCProfileConfig),
}

#[derive(PartialEq, Debug)]
pub struct CustomRCCProfileConfig {
    pub name: String,
    pub path: Utf8PathBuf,
}

#[derive(Clone)]
pub struct Plan {
    pub id: String,
    pub source: Source,
    pub working_directory: Utf8PathBuf,
    pub results_file: Utf8PathBuf,
    pub timeout: u64,
    pub robot: Robot,
    pub environment: Environment,
    pub session: Session,
    pub working_directory_cleanup_config: external_config::WorkingDirectoryCleanupConfig,
    pub cancellation_token: CancellationToken,
    pub host: Host,
    pub results_directory_locker: Locker,
    pub metadata: external_config::PlanMetadata,
    pub group_affiliation: GroupAffiliation,
}

#[derive(Clone)]
pub enum Source {
    Manual,
    Managed {
        zip_file: Utf8PathBuf,
        target: Utf8PathBuf,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct GroupAffiliation {
    pub group_index: usize,
    pub position_in_group: usize,
    pub execution_interval: u64,
}

pub fn from_external_config(
    external_config: external_config::Config,
    cancellation_token: CancellationToken,
    results_directory_locker: Locker,
) -> (GlobalConfig, Vec<Plan>) {
    let mut plans = vec![];

    for (group_index, sequential_group) in external_config.plan_groups.into_iter().enumerate() {
        for (plan_index, plan_config) in sequential_group.plans.into_iter().enumerate() {
            let (plan_source_dir, source) = match plan_config.source {
                external_config::Source::Manual { base_dir } => (base_dir.into(), Source::Manual),
                external_config::Source::Managed { zip_file } => {
                    let target = external_config
                        .managed_directory
                        .as_ref()
                        .join(&plan_config.id);
                    (
                        target.clone(),
                        Source::Managed {
                            zip_file: zip_file.into(),
                            target,
                        },
                    )
                }
            };
            plans.push(Plan {
                id: plan_config.id.clone(),
                source,
                working_directory: external_config
                    .working_directory
                    .as_ref()
                    .join("plans")
                    .join(&plan_config.id),
                results_file: plan_results_directory(&external_config.results_directory)
                    .join(format!("{}.json", plan_config.id)),
                timeout: plan_config.execution_config.timeout,
                robot: Robot::new(
                    RobotConfig {
                        robot_target: plan_source_dir.join(plan_config.robot_config.robot_target),
                        top_level_suite_name: plan_config.robot_config.top_level_suite_name,
                        suites: plan_config.robot_config.suites,
                        tests: plan_config.robot_config.tests,
                        test_tags_include: plan_config.robot_config.test_tags_include,
                        test_tags_exclude: plan_config.robot_config.test_tags_exclude,
                        variables: plan_config.robot_config.variables,
                        variable_files: plan_config
                            .robot_config
                            .variable_files
                            .into_iter()
                            .map(|f| plan_source_dir.join(f))
                            .collect(),
                        argument_files: plan_config
                            .robot_config
                            .argument_files
                            .into_iter()
                            .map(|f| plan_source_dir.join(f))
                            .collect(),
                        exit_on_failure: plan_config.robot_config.exit_on_failure,
                    },
                    plan_config.execution_config.n_attempts_max,
                    plan_config.execution_config.retry_strategy,
                ),
                environment: Environment::new(
                    &plan_source_dir,
                    &plan_config.id,
                    &external_config.rcc_config.binary_path,
                    &plan_config.environment_config,
                ),
                session: Session::new(&plan_config.session_config),
                working_directory_cleanup_config: plan_config.working_directory_cleanup_config,
                cancellation_token: cancellation_token.clone(),
                host: plan_config.host,
                results_directory_locker: results_directory_locker.clone(),
                metadata: plan_config.metadata,
                group_affiliation: GroupAffiliation {
                    group_index,
                    position_in_group: plan_index,
                    execution_interval: sequential_group.execution_interval,
                },
            });
        }
    }
    (
        GlobalConfig {
            working_directory: external_config.working_directory.into(),
            results_directory: external_config.results_directory.into(),
            managed_directory: external_config.managed_directory.into(),
            rcc_config: RCCConfig {
                binary_path: external_config.rcc_config.binary_path.into(),
                profile_config: match external_config.rcc_config.profile_config {
                    external_config::RCCProfileConfig::Default => RCCProfileConfig::Default,
                    external_config::RCCProfileConfig::Custom(custom) => {
                        RCCProfileConfig::Custom(CustomRCCProfileConfig {
                            name: custom.name,
                            path: custom.path.into(),
                        })
                    }
                },
            },
            cancellation_token,
            results_directory_locker,
        },
        plans,
    )
}

pub fn sort_plans_by_grouping(plans: &mut [Plan]) {
    plans.sort_by_key(|plan| {
        (
            plan.group_affiliation.group_index,
            plan.group_affiliation.position_in_group,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use robotmk::environment::{Environment, RCCEnvironment, SystemEnvironment};

    #[cfg(unix)]
    fn absolutify(path: &str) -> String {
        format!("/{path}")
    }

    #[cfg(windows)]
    fn absolutify(path: &str) -> String {
        format!("C:\\{path}")
    }

    fn system_plan_config() -> external_config::PlanConfig {
        external_config::PlanConfig {
            id: "system".into(),
            source: external_config::Source::Manual {
                base_dir: Utf8PathBuf::from(absolutify("synthetic_tests/system/"))
                    .try_into()
                    .unwrap(),
            },
            robot_config: external_config::RobotConfig {
                robot_target: Utf8PathBuf::from("tasks.robot").try_into().unwrap(),
                top_level_suite_name: Some("top_suite".into()),
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec![],
                argument_files: vec![
                    Utf8PathBuf::from("args.txt").try_into().unwrap(),
                    Utf8PathBuf::from("more_args.txt").try_into().unwrap(),
                ],
                exit_on_failure: false,
            },
            execution_config: external_config::ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: external_config::RetryStrategy::Incremental,
                timeout: 60,
            },
            environment_config: external_config::EnvironmentConfig::System,
            session_config: external_config::SessionConfig::Current,
            working_directory_cleanup_config:
                external_config::WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
            host: Host::Source,
            metadata: external_config::PlanMetadata {
                application: "sys_app".into(),
                suite_name: "my_first_suite".into(),
                variant: "".into(),
            },
        }
    }

    fn rcc_plan_config() -> external_config::PlanConfig {
        external_config::PlanConfig {
            id: "rcc".into(),
            source: external_config::Source::Manual {
                base_dir: Utf8PathBuf::from(absolutify("synthetic_tests/rcc/"))
                    .try_into()
                    .unwrap(),
            },
            robot_config: external_config::RobotConfig {
                robot_target: Utf8PathBuf::from("tasks.robot").try_into().unwrap(),
                top_level_suite_name: None,
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec![Utf8PathBuf::from("vars.txt").try_into().unwrap()],
                argument_files: vec![],
                exit_on_failure: false,
            },
            execution_config: external_config::ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: external_config::RetryStrategy::Complete,
                timeout: 60,
            },
            environment_config: external_config::EnvironmentConfig::Rcc(
                external_config::RCCEnvironmentConfig {
                    robot_yaml_path: Utf8PathBuf::from("robot.yaml").try_into().unwrap(),
                    build_timeout: 300,
                },
            ),
            session_config: external_config::SessionConfig::SpecificUser(
                external_config::UserSessionConfig {
                    user_name: "user".into(),
                },
            ),
            working_directory_cleanup_config:
                external_config::WorkingDirectoryCleanupConfig::MaxExecutions(50),
            host: Host::Source,
            metadata: external_config::PlanMetadata {
                application: "rcc_app".into(),
                suite_name: "my_second_suite".into(),
                variant: "".into(),
            },
        }
    }

    #[test]
    fn test_from_external_config() {
        let cancellation_token = CancellationToken::new();
        let (global_config, plans) = from_external_config(
            external_config::Config {
                working_directory: Utf8PathBuf::from(absolutify("working")).try_into().unwrap(),
                results_directory: Utf8PathBuf::from(absolutify("results")).try_into().unwrap(),
                managed_directory: Utf8PathBuf::from(absolutify("managed_robots"))
                    .try_into()
                    .unwrap(),
                rcc_config: external_config::RCCConfig {
                    binary_path: Utf8PathBuf::from(absolutify("bin/rcc")).try_into().unwrap(),
                    profile_config: external_config::RCCProfileConfig::Custom(
                        external_config::CustomRCCProfileConfig {
                            name: "Robotmk".into(),
                            path: Utf8PathBuf::from(absolutify("rcc_profile_robotmk.yaml"))
                                .try_into()
                                .unwrap(),
                        },
                    ),
                },
                plan_groups: vec![
                    external_config::SequentialPlanGroup {
                        plans: vec![rcc_plan_config()],
                        execution_interval: 300,
                    },
                    external_config::SequentialPlanGroup {
                        plans: vec![system_plan_config()],
                        execution_interval: 300,
                    },
                ],
            },
            cancellation_token.clone(),
            Locker::new("/config.json", Some(&cancellation_token)),
        );
        assert_eq!(global_config.working_directory, absolutify("working"));
        assert_eq!(global_config.results_directory, absolutify("results"));
        assert_eq!(
            global_config.rcc_config,
            RCCConfig {
                binary_path: Utf8PathBuf::from(absolutify("bin/rcc")),
                profile_config: RCCProfileConfig::Custom(CustomRCCProfileConfig {
                    name: "Robotmk".into(),
                    path: absolutify("rcc_profile_robotmk.yaml").into(),
                }),
            }
        );
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].id, "rcc");
        assert_eq!(plans[0].working_directory, absolutify("working/plans/rcc"));
        assert_eq!(plans[0].results_file, absolutify("results/plans/rcc.json"));
        assert_eq!(plans[0].timeout, 60);
        assert_eq!(
            plans[0].robot,
            Robot {
                robot_target: Utf8PathBuf::from(absolutify("synthetic_tests/rcc/tasks.robot")),
                command_line_args: vec![
                    "--variablefile".into(),
                    absolutify("synthetic_tests/rcc/vars.txt")
                ],
                n_attempts_max: 1,
                retry_strategy: external_config::RetryStrategy::Complete,
            }
        );
        assert_eq!(
            plans[0].environment,
            Environment::Rcc(RCCEnvironment {
                binary_path: Utf8PathBuf::from(absolutify("bin/rcc")),
                robot_yaml_path: Utf8PathBuf::from(absolutify("synthetic_tests/rcc/robot.yaml")),
                controller: "robotmk".into(),
                space: "rcc".into(),
                build_timeout: 300,
            })
        );
        assert_eq!(
            plans[0].working_directory_cleanup_config,
            external_config::WorkingDirectoryCleanupConfig::MaxExecutions(50),
        );
        assert_eq!(
            plans[0].metadata,
            external_config::PlanMetadata {
                application: "rcc_app".into(),
                suite_name: "my_second_suite".into(),
                variant: "".into(),
            },
        );
        assert_eq!(
            plans[0].group_affiliation,
            GroupAffiliation {
                group_index: 0,
                position_in_group: 0,
                execution_interval: 300,
            }
        );
        assert_eq!(plans[1].id, "system");
        assert_eq!(
            plans[1].working_directory,
            absolutify("working/plans/system")
        );
        assert_eq!(
            plans[1].results_file,
            absolutify("results/plans/system.json")
        );
        assert_eq!(plans[1].timeout, 60);
        assert_eq!(
            plans[1].robot,
            Robot {
                robot_target: Utf8PathBuf::from(absolutify("synthetic_tests/system/tasks.robot")),
                command_line_args: vec![
                    "--name".into(),
                    "top_suite".into(),
                    "--argumentfile".into(),
                    absolutify("synthetic_tests/system/args.txt"),
                    "--argumentfile".into(),
                    absolutify("synthetic_tests/system/more_args.txt")
                ],
                n_attempts_max: 1,
                retry_strategy: external_config::RetryStrategy::Incremental,
            }
        );
        assert_eq!(
            plans[1].environment,
            Environment::System(SystemEnvironment {})
        );
        assert_eq!(
            plans[1].working_directory_cleanup_config,
            external_config::WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
        );
        assert_eq!(
            plans[1].metadata,
            external_config::PlanMetadata {
                application: "sys_app".into(),
                suite_name: "my_first_suite".into(),
                variant: "".into(),
            },
        );
        assert_eq!(
            plans[1].group_affiliation,
            GroupAffiliation {
                group_index: 1,
                position_in_group: 0,
                execution_interval: 300,
            }
        );
    }
}
