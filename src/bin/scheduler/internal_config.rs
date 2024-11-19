use robotmk::config::{
    Config, PlanMetadata, RCCConfig, RobotConfig, Source as ConfigSource,
    WorkingDirectoryCleanupConfig,
};
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::{plan_results_directory, results_directory};
use robotmk::rf::robot::Robot;
use robotmk::section::Host;
use robotmk::session::Session;

use camino::Utf8PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct GlobalConfig {
    pub runtime_base_directory: Utf8PathBuf,
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub managed_directory: Utf8PathBuf,
    pub working_directory_plans: Utf8PathBuf,
    pub working_directory_environment_building: Utf8PathBuf,
    pub working_directory_rcc_setup_steps: Utf8PathBuf,
    pub rcc_config: RCCConfig,
    pub cancellation_token: CancellationToken,
    pub results_directory_locker: Locker,
}

#[derive(Clone)]
pub enum Source {
    Manual,
    Managed {
        tar_gz_path: Utf8PathBuf,
        target: Utf8PathBuf,
        version_number: usize,
        version_label: String,
    },
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
    pub working_directory_cleanup_config: WorkingDirectoryCleanupConfig,
    pub cancellation_token: CancellationToken,
    pub host: Host,
    pub results_directory_locker: Locker,
    pub metadata: PlanMetadata,
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
    cancellation_token: &CancellationToken,
    results_directory_locker: &Locker,
) -> (GlobalConfig, Vec<Plan>) {
    let working_directory = external_config.runtime_directory.join("working");
    let global_config = GlobalConfig {
        runtime_base_directory: external_config.runtime_directory.clone(),
        working_directory: working_directory.clone(),
        results_directory: results_directory(&external_config.runtime_directory),
        managed_directory: external_config.runtime_directory.join("managed"),
        working_directory_plans: working_directory.join("plans"),
        working_directory_environment_building: working_directory.join("environment_building"),
        working_directory_rcc_setup_steps: working_directory.join("rcc_setup"),
        rcc_config: external_config.rcc_config,
        cancellation_token: cancellation_token.clone(),
        results_directory_locker: results_directory_locker.clone(),
    };

    let mut plans = vec![];
    for (group_index, sequential_group) in external_config.plan_groups.into_iter().enumerate() {
        for (plan_index, plan_config) in sequential_group.plans.into_iter().enumerate() {
            let (plan_source_dir, source) = match &plan_config.source {
                ConfigSource::Manual { base_dir } => (base_dir.clone(), Source::Manual),
                ConfigSource::Managed {
                    tar_gz_path,
                    version_number,
                    version_label,
                } => {
                    let target = global_config.managed_directory.join(&plan_config.id);
                    (
                        target.clone(),
                        Source::Managed {
                            tar_gz_path: tar_gz_path.clone(),
                            target,
                            version_number: *version_number,
                            version_label: version_label.clone(),
                        },
                    )
                }
            };
            let session = Session::new(&plan_config.session_config);
            plans.push(Plan {
                id: plan_config.id.clone(),
                source,
                working_directory: global_config.working_directory_plans.join(&plan_config.id),
                results_file: plan_results_directory(&global_config.results_directory)
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
                    &session.robocorp_home(&global_config.rcc_config.robocorp_home_base),
                    &plan_config.id,
                    &global_config.rcc_config.binary_path,
                    &plan_config.environment_config,
                    &global_config
                        .working_directory_environment_building
                        .join(&plan_config.id),
                ),
                session,
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
    (global_config, plans)
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
    use robotmk::config::{
        CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, PlanConfig,
        RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup,
        SessionConfig,
    };
    use robotmk::environment::{Environment, RCCEnvironment, SystemEnvironment};

    fn system_plan_config() -> PlanConfig {
        PlanConfig {
            id: "system".into(),
            source: ConfigSource::Manual {
                base_dir: "/synthetic_tests/system/".into(),
            },
            robot_config: RobotConfig {
                robot_target: Utf8PathBuf::from("tasks.robot"),
                top_level_suite_name: Some("top_suite".into()),
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec![],
                argument_files: vec!["args.txt".into(), "more_args.txt".into()],
                exit_on_failure: false,
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
            metadata: PlanMetadata {
                application: "sys_app".into(),
                suite_name: "my_first_suite".into(),
                variant: "".into(),
            },
        }
    }

    fn rcc_plan_config() -> PlanConfig {
        PlanConfig {
            id: "rcc".into(),
            source: ConfigSource::Manual {
                base_dir: "/synthetic_tests/rcc/".into(),
            },
            robot_config: RobotConfig {
                robot_target: Utf8PathBuf::from("tasks.robot"),
                top_level_suite_name: None,
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec!["vars.txt".into()],
                argument_files: vec![],
                exit_on_failure: false,
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                robot_yaml_path: Utf8PathBuf::from("robot.yaml"),
                build_timeout: 300,
                remote_origin: None,
                catalog_zip: None,
            }),
            #[cfg(unix)]
            session_config: SessionConfig::Current,
            #[cfg(windows)]
            session_config: SessionConfig::SpecificUser(robotmk::config::UserSessionConfig {
                user_name: "user".into(),
            }),
            working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(50),
            host: Host::Source,
            metadata: PlanMetadata {
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
            Config {
                runtime_directory: Utf8PathBuf::from("/"),
                rcc_config: RCCConfig {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    profile_config: RCCProfileConfig::Custom(CustomRCCProfileConfig {
                        name: "Robotmk".into(),
                        path: "/rcc_profile_robotmk.yaml".into(),
                    }),
                    robocorp_home_base: Utf8PathBuf::from("/rc_home_base"),
                },
                plan_groups: vec![
                    SequentialPlanGroup {
                        plans: vec![rcc_plan_config()],
                        execution_interval: 300,
                    },
                    SequentialPlanGroup {
                        plans: vec![system_plan_config()],
                        execution_interval: 300,
                    },
                ],
            },
            &cancellation_token,
            &Locker::new("/config.json", Some(&cancellation_token)),
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
                robocorp_home_base: Utf8PathBuf::from("/rc_home_base"),
            }
        );
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].id, "rcc");
        assert_eq!(plans[0].working_directory, "/working/plans/rcc");
        assert_eq!(plans[0].results_file, "/results/plans/rcc.json");
        assert_eq!(plans[0].timeout, 60);
        assert_eq!(
            plans[0].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/synthetic_tests/rcc/tasks.robot"),
                command_line_args: vec![
                    "--variablefile".into(),
                    "/synthetic_tests/rcc/vars.txt".into()
                ],
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
            }
        );
        assert_eq!(
            plans[0].environment,
            Environment::Rcc(RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                remote_origin: None,
                catalog_zip: None,
                robot_yaml_path: Utf8PathBuf::from("/synthetic_tests/rcc/robot.yaml"),
                controller: "robotmk".into(),
                space: "rcc".into(),
                build_timeout: 300,
                build_runtime_directory: Utf8PathBuf::from("/working/environment_building/rcc"),
                #[cfg(unix)]
                robocorp_home: Utf8PathBuf::from("/rc_home_base")
                    .join("current_user")
                    .to_string(),
                #[cfg(windows)]
                robocorp_home: Utf8PathBuf::from("/rc_home_base")
                    .join("user_user")
                    .to_string(),
            })
        );
        assert_eq!(
            plans[0].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxExecutions(50),
        );
        assert_eq!(
            plans[0].metadata,
            PlanMetadata {
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
        assert_eq!(plans[1].working_directory, "/working/plans/system");
        assert_eq!(plans[1].results_file, "/results/plans/system.json");
        assert_eq!(plans[1].timeout, 60);
        assert_eq!(
            plans[1].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/synthetic_tests/system/tasks.robot"),
                command_line_args: vec![
                    "--name".into(),
                    "top_suite".into(),
                    "--argumentfile".into(),
                    "/synthetic_tests/system/args.txt".into(),
                    "--argumentfile".into(),
                    "/synthetic_tests/system/more_args.txt".into()
                ],
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Incremental,
            }
        );
        assert_eq!(
            plans[1].environment,
            Environment::System(SystemEnvironment {})
        );
        assert_eq!(
            plans[1].working_directory_cleanup_config,
            WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
        );
        assert_eq!(
            plans[1].metadata,
            PlanMetadata {
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
