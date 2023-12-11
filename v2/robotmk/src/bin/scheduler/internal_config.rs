use crate::rf::robot::Robot;
use crate::sessions::session::Session;
use robotmk::command_spec::CommandSpec;
use robotmk::config::EnvironmentConfig;
use robotmk::config::{Config, RCCConfig, WorkingDirectoryCleanupConfig};
use robotmk::environment::ResultCode;
use robotmk::lock::Locker;
use robotmk::results::suite_results_directory;
use robotmk::section::Host;

use camino::{Utf8Path, Utf8PathBuf};
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
                robot_target: suite_config.robot_framework_config.robot_target,
                command_line_args: suite_config.robot_framework_config.command_line_args,
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

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Environment {
    System(SystemEnvironment),
    Rcc(RCCEnvironment),
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct SystemEnvironment {}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct RCCEnvironment {
    pub binary_path: Utf8PathBuf,
    pub robot_yaml_path: Utf8PathBuf,
    pub controller: String,
    pub space: String,
    pub build_timeout: u64,
    pub env_json_path: Option<Utf8PathBuf>,
}

impl Environment {
    pub fn new(
        suite_id: &str,
        rcc_binary_path: &Utf8Path,
        environment_config: &EnvironmentConfig,
    ) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_binary_path.to_path_buf(),
                robot_yaml_path: rcc_environment_config.robot_yaml_path.clone(),
                controller: String::from("robotmk"),
                space: suite_id.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
                env_json_path: rcc_environment_config.env_json_path.clone(),
            }),
        }
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        match self {
            Self::System(system_environment) => system_environment.wrap(command_spec),
            Self::Rcc(rcc_environment) => rcc_environment.wrap(command_spec),
        }
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        match self {
            Self::System(_) => SystemEnvironment::create_result_code(exit_code),
            Self::Rcc(_) => RCCEnvironment::create_result_code(exit_code),
        }
    }
}

impl SystemEnvironment {
    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        command_spec
    }

    fn create_result_code(exit_code: i32) -> ResultCode {
        if exit_code == 0 {
            return ResultCode::AllTestsPassed;
        }
        ResultCode::RobotCommandFailed
    }
}

impl RCCEnvironment {
    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec = CommandSpec::new(&self.binary_path);
        wrapped_spec
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build");
        apply_current_settings(
            &self.robot_yaml_path,
            &self.controller,
            &self.space,
            self.env_json_path.as_deref(),
            &mut wrapped_spec,
        );
        wrapped_spec
            .add_argument("--")
            .add_argument(command_spec.executable)
            .add_arguments(command_spec.arguments);
        wrapped_spec
    }

    fn create_result_code(exit_code: i32) -> ResultCode {
        match exit_code {
            0 => ResultCode::AllTestsPassed,
            10 => ResultCode::RobotCommandFailed,
            _ => ResultCode::EnvironmentFailed,
        }
    }
}

pub fn apply_current_settings(
    robot_yaml_path: &Utf8Path,
    controller: &str,
    space: &str,
    env_json_path: Option<&Utf8Path>,
    command_spec: &mut CommandSpec,
) {
    command_spec
        .add_argument("--robot")
        .add_argument(robot_yaml_path)
        .add_argument("--controller")
        .add_argument(controller)
        .add_argument("--space")
        .add_argument(space);
    if let Some(env_json_path) = &env_json_path {
        command_spec
            .add_argument("--environment")
            .add_argument(env_json_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robotmk::config::RCCEnvironmentConfig;

    fn command_spec_for_wrap() -> CommandSpec {
        let mut command_spec = CommandSpec::new("C:\\x\\y\\z.exe");
        command_spec
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        command_spec
    }

    #[test]
    fn test_system_wrap() {
        assert_eq!(
            SystemEnvironment {}.wrap(command_spec_for_wrap()),
            command_spec_for_wrap()
        );
    }

    #[test]
    fn test_rcc_wrap() {
        let mut expected = CommandSpec::new("C:\\bin\\z.exe");
        expected
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build")
            .add_argument("--robot")
            .add_argument("C:\\my_suite\\robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_suite")
            .add_argument("--environment")
            .add_argument("C:\\my_suite\\env.json")
            .add_argument("--")
            .add_argument("C:\\x\\y\\z.exe")
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                robot_yaml_path: Utf8PathBuf::from("C:\\my_suite\\robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_suite"),
                build_timeout: 600,
                env_json_path: Some("C:\\my_suite\\env.json".into())
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }

    use crate::sessions::session::{CurrentSession, UserSession};
    use robotmk::config::{
        EnvironmentConfig, ExecutionConfig, RCCProfileConfig, RetryStrategy, RobotFrameworkConfig,
        SessionConfig, SuiteConfig, UserSessionConfig,
    };

    use std::collections::HashMap;

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
        let cancellation_token = CancellationToken::new();
        let (global_config, suites) = from_external_config(
            Config {
                working_directory: Utf8PathBuf::from("/working"),
                results_directory: Utf8PathBuf::from("/results"),
                rcc_config: RCCConfig {
                    binary_path: Utf8PathBuf::from("/bin/rcc"),
                    profile_config: Some(RCCProfileConfig {
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
                profile_config: Some(RCCProfileConfig {
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
