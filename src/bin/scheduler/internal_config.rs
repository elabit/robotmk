use robotmk::config;
use robotmk::env::{
    Environment,
    conda::{CondaEnvironmentFromArchive, CondaEnvironmentFromManifest},
    rcc::RCCEnvironment,
    system::SystemEnvironment,
};
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
    pub rcc_config: config::RCCConfig,
    pub conda_config: CondaConfig,
    pub cancellation_token: CancellationToken,
    pub results_directory_locker: Locker,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CondaConfig {
    pub micromamba_binary_path: Utf8PathBuf,
    pub base_directory: Utf8PathBuf,
}

impl CondaConfig {
    pub fn root_prefix(&self) -> Utf8PathBuf {
        self.base_directory.join("mamba_root_prefix")
    }

    pub fn environments_base_directory(&self) -> Utf8PathBuf {
        self.base_directory.join("environments")
    }
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
    pub working_directory_cleanup_config: config::WorkingDirectoryCleanupConfig,
    pub cancellation_token: CancellationToken,
    pub host: Host,
    pub results_directory_locker: Locker,
    pub metadata: config::PlanMetadata,
    pub group_affiliation: GroupAffiliation,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GroupAffiliation {
    pub group_index: usize,
    pub position_in_group: usize,
    pub execution_interval: u64,
}

pub fn from_external_config(
    external_config: config::Config,
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
        conda_config: CondaConfig {
            micromamba_binary_path: external_config.conda_config.micromamba_binary_path.into(),
            base_directory: external_config.conda_config.base_directory,
        },
        cancellation_token: cancellation_token.clone(),
        results_directory_locker: results_directory_locker.clone(),
    };

    let mut plans = vec![];
    for (group_index, sequential_group) in external_config.plan_groups.into_iter().enumerate() {
        for (plan_index, plan_config) in sequential_group.plans.into_iter().enumerate() {
            let (plan_source_dir, source) = match &plan_config.source {
                config::Source::Manual { base_dir } => (base_dir.clone(), Source::Manual),
                config::Source::Managed {
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
                    config::RobotConfig {
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
                        environment_variables_rendered_obfuscated: plan_config
                            .robot_config
                            .environment_variables_rendered_obfuscated,
                    },
                    plan_config.execution_config.n_attempts_max,
                    plan_config.execution_config.retry_strategy,
                ),
                environment: match plan_config.environment_config {
                    config::EnvironmentConfig::System => Environment::System(SystemEnvironment {}),
                    config::EnvironmentConfig::Rcc(rcc_environment_config) => {
                        Environment::Rcc(RCCEnvironment::new(
                            &plan_source_dir,
                            &session.robocorp_home(&global_config.rcc_config.robocorp_home_base),
                            &plan_config.id,
                            &global_config.rcc_config.binary_path,
                            &rcc_environment_config,
                            &global_config
                                .working_directory_environment_building
                                .join(&plan_config.id),
                        ))
                    }
                    config::EnvironmentConfig::Conda(conda_environment_config) => {
                        match conda_environment_config.source {
                            config::CondaEnvironmentSource::Manifest(conda_env_from_manifest) => {
                                Environment::CondaFromManifest(CondaEnvironmentFromManifest {
                                    micromamba_binary_path: global_config
                                        .conda_config
                                        .micromamba_binary_path
                                        .clone(),
                                    manifest_path: plan_source_dir
                                        .join(conda_env_from_manifest.manifest_path),
                                    root_prefix: global_config.conda_config.root_prefix(),
                                    prefix: global_config
                                        .conda_config
                                        .environments_base_directory()
                                        .join(&plan_config.id),
                                    http_proxy_config: conda_env_from_manifest.http_proxy_config,
                                    build_timeout: conda_environment_config.build_timeout,
                                    build_runtime_directory: global_config
                                        .working_directory_environment_building
                                        .join(&plan_config.id),
                                })
                            }
                            config::CondaEnvironmentSource::Archive(archive_path) => {
                                Environment::CondaFromArchive(CondaEnvironmentFromArchive {
                                    micromamba_binary_path: global_config
                                        .conda_config
                                        .micromamba_binary_path
                                        .clone(),
                                    archive_path,
                                    root_prefix: global_config.conda_config.root_prefix(),
                                    prefix: global_config
                                        .conda_config
                                        .environments_base_directory()
                                        .join(&plan_config.id),
                                    build_timeout: conda_environment_config.build_timeout,
                                    build_runtime_directory: global_config
                                        .working_directory_environment_building
                                        .join(&plan_config.id),
                                })
                            }
                        }
                    }
                },
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
    use robotmk::session::CurrentSession;
    #[cfg(windows)]
    use robotmk::session::UserSession;

    fn system_plan_config() -> config::PlanConfig {
        config::PlanConfig {
            id: "system".into(),
            source: config::Source::Manual {
                base_dir: "/synthetic_tests/system/".into(),
            },
            robot_config: config::RobotConfig {
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
                environment_variables_rendered_obfuscated: vec![],
            },
            execution_config: config::ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Incremental,
                timeout: 60,
            },
            environment_config: config::EnvironmentConfig::System,
            session_config: config::SessionConfig::Current,
            working_directory_cleanup_config: config::WorkingDirectoryCleanupConfig::MaxAgeSecs(
                1209600,
            ),
            host: Host::Source,
            metadata: config::PlanMetadata {
                application: "sys_app".into(),
                suite_name: "my_first_suite".into(),
                variant: "".into(),
            },
        }
    }

    fn rcc_plan_config() -> config::PlanConfig {
        config::PlanConfig {
            id: "rcc".into(),
            source: config::Source::Manual {
                base_dir: "/synthetic_tests/rcc/".into(),
            },
            robot_config: config::RobotConfig {
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
                environment_variables_rendered_obfuscated: vec![],
            },
            execution_config: config::ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Complete,
                timeout: 60,
            },
            environment_config: config::EnvironmentConfig::Rcc(config::RCCEnvironmentConfig {
                robot_yaml_path: Utf8PathBuf::from("robot.yaml"),
                build_timeout: 300,
                remote_origin: None,
                catalog_zip: None,
            }),
            #[cfg(unix)]
            session_config: config::SessionConfig::Current,
            #[cfg(windows)]
            session_config: config::SessionConfig::SpecificUser(config::UserSessionConfig {
                user_name: "user".into(),
            }),
            working_directory_cleanup_config: config::WorkingDirectoryCleanupConfig::MaxExecutions(
                50,
            ),
            host: Host::Source,
            metadata: config::PlanMetadata {
                application: "rcc_app".into(),
                suite_name: "my_second_suite".into(),
                variant: "".into(),
            },
        }
    }

    fn conda_manifest_plan_config() -> config::PlanConfig {
        config::PlanConfig {
            id: "app1_suite1".into(),
            source: config::Source::Managed {
                tar_gz_path: Utf8PathBuf::from("/synthetic_tests/app1_suite1.tar.gz"),
                version_number: 1,
                version_label: "label".into(),
            },
            robot_config: config::RobotConfig {
                robot_target: Utf8PathBuf::from("app1/tasks.robot"),
                top_level_suite_name: Some("suite1".into()),
                suites: vec![],
                tests: vec!["test1".into()],
                test_tags_include: vec![],
                test_tags_exclude: vec![],
                variables: vec![],
                variable_files: vec!["app1/vars.txt".into()],
                argument_files: vec![],
                exit_on_failure: true,
                environment_variables_rendered_obfuscated: vec![],
            },
            execution_config: config::ExecutionConfig {
                n_attempts_max: 2,
                retry_strategy: config::RetryStrategy::Incremental,
                timeout: 60,
            },
            environment_config: config::EnvironmentConfig::Conda(config::CondaEnvironmentConfig {
                source: config::CondaEnvironmentSource::Manifest(
                    config::CondaEnvironmentFromManifest {
                        manifest_path: "app1/app1_env.yaml".into(),
                        http_proxy_config: config::HTTPProxyConfig {
                            http: None,
                            https: Some("http://user:pass@corp.com:8080".into()),
                        },
                    },
                ),
                build_timeout: 300,
            }),
            session_config: config::SessionConfig::Current,
            working_directory_cleanup_config: config::WorkingDirectoryCleanupConfig::MaxExecutions(
                5,
            ),
            host: Host::Source,
            metadata: config::PlanMetadata {
                application: "app1".into(),
                suite_name: "suite1".into(),
                variant: "".into(),
            },
        }
    }

    fn conda_archive_plan_config() -> config::PlanConfig {
        config::PlanConfig {
            id: "app2_tests_EN".into(),
            source: config::Source::Manual {
                base_dir: Utf8PathBuf::from("/app2"),
            },
            robot_config: config::RobotConfig {
                robot_target: Utf8PathBuf::from("tests"),
                top_level_suite_name: None,
                suites: vec![],
                tests: vec![],
                test_tags_include: vec![],
                test_tags_exclude: vec!["experimental".into()],
                variables: vec![config::RobotFrameworkVariable {
                    name: "var1".into(),
                    value: "value1".into(),
                }],
                variable_files: vec![],
                argument_files: vec![],
                exit_on_failure: false,
                environment_variables_rendered_obfuscated: vec![
                    config::RobotFrameworkObfuscatedEnvVar {
                        name: "env1".into(),
                        value: "value1".into(),
                    },
                ],
            },
            execution_config: config::ExecutionConfig {
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Complete,
                timeout: 60,
            },
            environment_config: config::EnvironmentConfig::Conda(config::CondaEnvironmentConfig {
                source: config::CondaEnvironmentSource::Archive(Utf8PathBuf::from(
                    "/app2.env.tar.gz",
                )),
                build_timeout: 300,
            }),
            #[cfg(unix)]
            session_config: config::SessionConfig::Current,
            #[cfg(windows)]
            session_config: config::SessionConfig::SpecificUser(config::UserSessionConfig {
                user_name: "user".into(),
            }),
            working_directory_cleanup_config: config::WorkingDirectoryCleanupConfig::MaxExecutions(
                5,
            ),
            host: Host::Piggyback("piggy".into()),
            metadata: config::PlanMetadata {
                application: "app2".into(),
                suite_name: "tests".into(),
                variant: "EN".into(),
            },
        }
    }

    #[test]
    fn test_from_external_config() {
        let cancellation_token = CancellationToken::new();
        let (global_config, plans) = from_external_config(
            config::Config {
                runtime_directory: Utf8PathBuf::from("/"),
                rcc_config: config::RCCConfig {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    profile_config: config::RCCProfileConfig::Custom(
                        config::CustomRCCProfileConfig {
                            name: "Robotmk".into(),
                            path: "/rcc_profile_robotmk.yaml".into(),
                        },
                    ),
                    robocorp_home_base: Utf8PathBuf::from("/rc_home_base"),
                },
                conda_config: config::CondaConfig {
                    micromamba_binary_path: config::ValidatedMicromambaBinaryPath::try_from(
                        Utf8PathBuf::from(
                            #[cfg(unix)]
                            {
                                "/micromamba"
                            },
                            #[cfg(windows)]
                            {
                                "C:\\micromamba.exe"
                            },
                        ),
                    )
                    .unwrap(),
                    base_directory: Utf8PathBuf::from("/conda_base"),
                },
                plan_groups: vec![
                    config::SequentialPlanGroup {
                        plans: vec![rcc_plan_config()],
                        execution_interval: 300,
                    },
                    config::SequentialPlanGroup {
                        plans: vec![system_plan_config()],
                        execution_interval: 300,
                    },
                    config::SequentialPlanGroup {
                        plans: vec![conda_manifest_plan_config(), conda_archive_plan_config()],
                        execution_interval: 600,
                    },
                ],
            },
            &cancellation_token,
            &Locker::new("/config.json", Some(&cancellation_token)),
        );
        assert_eq!(global_config.working_directory, "/working");
        assert_eq!(global_config.results_directory, "/results");
        assert_eq!(global_config.managed_directory, "/managed");
        assert_eq!(
            global_config.rcc_config,
            config::RCCConfig {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                profile_config: config::RCCProfileConfig::Custom(config::CustomRCCProfileConfig {
                    name: "Robotmk".into(),
                    path: "/rcc_profile_robotmk.yaml".into(),
                }),
                robocorp_home_base: Utf8PathBuf::from("/rc_home_base"),
            }
        );
        assert_eq!(
            global_config.conda_config,
            CondaConfig {
                micromamba_binary_path: Utf8PathBuf::from(
                    #[cfg(unix)]
                    {
                        "/micromamba"
                    },
                    #[cfg(windows)]
                    {
                        "C:\\micromamba.exe"
                    },
                ),
                base_directory: Utf8PathBuf::from("/conda_base"),
            }
        );
        assert_eq!(plans.len(), 4);
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
                envs_rendered_obfuscated: vec![],
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Complete,
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
        #[cfg(unix)]
        assert_eq!(plans[0].session, Session::Current(CurrentSession {}),);
        #[cfg(windows)]
        assert_eq!(
            plans[0].session,
            Session::User(UserSession {
                user_name: "user".into()
            }),
        );
        assert_eq!(
            plans[0].working_directory_cleanup_config,
            config::WorkingDirectoryCleanupConfig::MaxExecutions(50),
        );
        assert_eq!(
            plans[0].metadata,
            config::PlanMetadata {
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
                envs_rendered_obfuscated: vec![],
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Incremental,
            }
        );
        assert_eq!(
            plans[1].environment,
            Environment::System(SystemEnvironment {})
        );
        assert_eq!(plans[1].session, Session::Current(CurrentSession {}),);
        assert_eq!(
            plans[1].working_directory_cleanup_config,
            config::WorkingDirectoryCleanupConfig::MaxAgeSecs(1209600),
        );
        assert_eq!(
            plans[1].metadata,
            config::PlanMetadata {
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
        assert_eq!(plans[2].id, "app1_suite1");
        assert_eq!(plans[2].working_directory, "/working/plans/app1_suite1");
        assert_eq!(plans[2].results_file, "/results/plans/app1_suite1.json");
        assert_eq!(plans[2].timeout, 60);
        assert_eq!(
            plans[2].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/managed")
                    .join("app1_suite1")
                    .join("app1/tasks.robot"),
                command_line_args: vec![
                    "--name".into(),
                    "suite1".into(),
                    "--test".into(),
                    "test1".into(),
                    "--variablefile".into(),
                    Utf8PathBuf::from("/managed")
                        .join("app1_suite1")
                        .join("app1/vars.txt")
                        .into(),
                    "--exitonfailure".into(),
                ],
                envs_rendered_obfuscated: vec![],
                n_attempts_max: 2,
                retry_strategy: config::RetryStrategy::Incremental,
            }
        );
        assert_eq!(
            plans[2].environment,
            Environment::CondaFromManifest(CondaEnvironmentFromManifest {
                micromamba_binary_path: Utf8PathBuf::from(
                    #[cfg(unix)]
                    {
                        "/micromamba"
                    },
                    #[cfg(windows)]
                    {
                        "C:\\micromamba.exe"
                    },
                ),
                manifest_path: Utf8PathBuf::from("/managed/app1_suite1/app1/app1_env.yaml"),
                root_prefix: Utf8PathBuf::from("/conda_base/mamba_root_prefix"),
                prefix: Utf8PathBuf::from("/conda_base/environments/app1_suite1"),
                http_proxy_config: config::HTTPProxyConfig {
                    http: None,
                    https: Some("http://user:pass@corp.com:8080".into()),
                },
                build_timeout: 300,
                build_runtime_directory: Utf8PathBuf::from(
                    "/working/environment_building/app1_suite1"
                ),
            })
        );
        assert_eq!(plans[2].session, Session::Current(CurrentSession {}),);
        assert_eq!(
            plans[2].working_directory_cleanup_config,
            config::WorkingDirectoryCleanupConfig::MaxExecutions(5),
        );
        assert_eq!(
            plans[2].metadata,
            config::PlanMetadata {
                application: "app1".into(),
                suite_name: "suite1".into(),
                variant: "".into(),
            },
        );
        assert_eq!(
            plans[2].group_affiliation,
            GroupAffiliation {
                group_index: 2,
                position_in_group: 0,
                execution_interval: 600,
            }
        );
        assert_eq!(plans[3].id, "app2_tests_EN");
        assert_eq!(plans[3].working_directory, "/working/plans/app2_tests_EN");
        assert_eq!(plans[3].results_file, "/results/plans/app2_tests_EN.json");
        assert_eq!(plans[3].timeout, 60);
        assert_eq!(
            plans[3].robot,
            Robot {
                robot_target: Utf8PathBuf::from("/app2/tests"),
                command_line_args: vec![
                    "--exclude".into(),
                    "experimental".into(),
                    "--variable".into(),
                    "var1:value1".into()
                ],
                envs_rendered_obfuscated: vec![("env1".into(), "value1".into())],
                n_attempts_max: 1,
                retry_strategy: config::RetryStrategy::Complete,
            }
        );
        assert_eq!(
            plans[3].environment,
            Environment::CondaFromArchive(CondaEnvironmentFromArchive {
                micromamba_binary_path: Utf8PathBuf::from(
                    #[cfg(unix)]
                    {
                        "/micromamba"
                    },
                    #[cfg(windows)]
                    {
                        "C:\\micromamba.exe"
                    },
                ),
                archive_path: Utf8PathBuf::from("/app2.env.tar.gz"),
                root_prefix: Utf8PathBuf::from("/conda_base/mamba_root_prefix"),
                prefix: Utf8PathBuf::from("/conda_base/environments/app2_tests_EN"),
                build_timeout: 300,
                build_runtime_directory: Utf8PathBuf::from(
                    "/working/environment_building/app2_tests_EN"
                ),
            })
        );
        #[cfg(unix)]
        assert_eq!(plans[3].session, Session::Current(CurrentSession {}),);
        #[cfg(windows)]
        assert_eq!(
            plans[3].session,
            Session::User(UserSession {
                user_name: "user".into()
            }),
        );
        assert_eq!(
            plans[3].working_directory_cleanup_config,
            config::WorkingDirectoryCleanupConfig::MaxExecutions(5),
        );
        assert_eq!(
            plans[3].metadata,
            config::PlanMetadata {
                application: "app2".into(),
                suite_name: "tests".into(),
                variant: "EN".into(),
            },
        );
        assert_eq!(
            plans[3].group_affiliation,
            GroupAffiliation {
                group_index: 2,
                position_in_group: 1,
                execution_interval: 600,
            }
        );
    }
}
