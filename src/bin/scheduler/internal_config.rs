use robotmk::config::{
    Config, PlanMetadata, RCCConfig, Source as ConfigSource, WorkingDirectoryCleanupConfig,
};
use robotmk::environment::Environment;
use robotmk::lock::Locker;
use robotmk::results::plan_results_directory;
use robotmk::rf::robot::Robot;
use robotmk::section::Host;
use robotmk::session::Session;

use camino::Utf8PathBuf;
use tokio_util::sync::CancellationToken;

pub struct GlobalConfig {
    pub working_directory: Utf8PathBuf,
    pub results_directory: Utf8PathBuf,
    pub managed_directory: Utf8PathBuf, // Absolute path
    pub rcc_config: RCCConfig,
    pub cancellation_token: CancellationToken,
    pub results_directory_locker: Locker,
}

#[derive(Clone)]
pub enum Source {
    Manual,
    Managed {
        zip_file: Utf8PathBuf,
        target: Utf8PathBuf,
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
    cancellation_token: CancellationToken,
    results_directory_locker: Locker,
) -> (GlobalConfig, Vec<Plan>) {
    let mut plans = vec![];

    for (group_index, sequential_group) in external_config.plan_groups.into_iter().enumerate() {
        for (plan_index, plan_config) in sequential_group.plans.into_iter().enumerate() {
            let (plan_source_dir, source) = match &plan_config.source {
                ConfigSource::Manual { base_dir } => (base_dir.clone(), Source::Manual),
                ConfigSource::Managed { zip_file } => {
                    let target = external_config.managed_directory.join(&plan_config.id);
                    (
                        target.clone(),
                        Source::Managed {
                            zip_file: zip_file.clone(),
                            target,
                        },
                    )
                }
            };
            let mut command_line_args = plan_config.robot_config.command_line_args.clone();
            for file in &plan_config.robot_config.argument_files {
                command_line_args.push("--argumentfile".into());
                command_line_args.push(plan_source_dir.join(file).into())
            }
            for file in &plan_config.robot_config.variable_files {
                command_line_args.push("--variablefile".into());
                command_line_args.push(plan_source_dir.join(file).into())
            }
            plans.push(Plan {
                id: plan_config.id.clone(),
                source,
                working_directory: external_config
                    .working_directory
                    .join("plans")
                    .join(&plan_config.id),
                results_file: plan_results_directory(&external_config.results_directory)
                    .join(format!("{}.json", plan_config.id)),
                timeout: plan_config.execution_config.timeout,
                robot: Robot {
                    robot_target: plan_source_dir.join(plan_config.robot_config.robot_target),
                    command_line_args,
                    n_attempts_max: plan_config.execution_config.n_attempts_max,
                    retry_strategy: plan_config.execution_config.retry_strategy,
                },
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
            working_directory: external_config.working_directory,
            results_directory: external_config.results_directory,
            managed_directory: external_config.managed_directory,
            rcc_config: external_config.rcc_config,
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
    use robotmk::config::{
        CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, PlanConfig,
        RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup,
        SessionConfig, UserSessionConfig,
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
                variable_files: vec![],
                argument_files: vec![],
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
                variable_files: vec![],
                argument_files: vec![],
                command_line_args: vec![],
            },
            execution_config: ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
                timeout: 60,
            },
            environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                robot_yaml_path: Utf8PathBuf::from("robot.yaml"),
                build_timeout: 300,
            }),
            session_config: SessionConfig::SpecificUser(UserSessionConfig {
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
                working_directory: Utf8PathBuf::from("/working"),
                results_directory: Utf8PathBuf::from("/results"),
                managed_directory: Utf8PathBuf::from("/managed_robots"),
                rcc_config: RCCConfig {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    profile_config: RCCProfileConfig::Custom(CustomRCCProfileConfig {
                        name: "Robotmk".into(),
                        path: "/rcc_profile_robotmk.yaml".into(),
                    }),
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
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].id, "rcc");
        assert_eq!(plans[0].working_directory, "/working/plans/rcc");
        assert_eq!(plans[0].results_file, "/results/plans/rcc.json");
        assert_eq!(plans[0].timeout, 60);
        assert_eq!(
            plans[0].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/synthetic_tests/rcc/tasks.robot"),
                command_line_args: vec![],
                n_attempts_max: 1,
                retry_strategy: RetryStrategy::Complete,
            }
        );
        assert_eq!(
            plans[0].environment,
            Environment::Rcc(RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                robot_yaml_path: Utf8PathBuf::from("/synthetic_tests/rcc/robot.yaml"),
                controller: "robotmk".into(),
                space: "rcc".into(),
                build_timeout: 300,
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
                command_line_args: vec![],
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
