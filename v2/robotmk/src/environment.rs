use crate::command_spec::CommandSpec;
use crate::config::EnvironmentConfig;

use camino::{Utf8Path, Utf8PathBuf};

pub enum ResultCode {
    AllTestsPassed,
    RobotCommandFailed,
    EnvironmentFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
    System(SystemEnvironment),
    Rcc(RCCEnvironment),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemEnvironment {}

#[derive(Clone, Debug, PartialEq)]
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
}
