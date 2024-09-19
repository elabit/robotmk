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
    pub remote_origin: Option<String>,
    pub robot_yaml_path: Utf8PathBuf,
    pub controller: String,
    pub space: String,
    pub build_timeout: u64,
}

impl Environment {
    pub fn new(
        base_dir: &Utf8Path,
        plan_id: &str,
        rcc_binary_path: &Utf8Path,
        environment_config: &EnvironmentConfig,
    ) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_binary_path.to_path_buf(),
                remote_origin: rcc_environment_config.remote_origin.clone(),
                robot_yaml_path: base_dir.join(&rcc_environment_config.robot_yaml_path),
                controller: String::from("robotmk"),
                space: plan_id.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
            }),
        }
    }

    pub fn build_instructions(&self) -> Option<BuildInstructions> {
        match self {
            Self::System(system_environment) => system_environment.build_instructions(),
            Self::Rcc(rcc_environment) => rcc_environment.build_instructions(),
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
    fn build_instructions(&self) -> Option<BuildInstructions> {
        None
    }

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
    pub fn bundled_command_spec(binary_path: &Utf8Path) -> CommandSpec {
        let mut command_spec = CommandSpec::new(binary_path);
        command_spec.add_argument("--bundled");
        command_spec
    }

    fn build_instructions(&self) -> Option<BuildInstructions> {
        let mut build_command_spec = Self::bundled_command_spec(&self.binary_path);
        build_command_spec
            .add_argument("task")
            .add_argument("script");
        self.apply_current_settings(&mut build_command_spec);
        if let Some(remote_origin) = &self.remote_origin {
            build_command_spec
                .add_env(String::from("RCC_REMOTE_ORIGIN"), remote_origin.to_string());
        }

        let mut version_command_spec = Self::bundled_command_spec(&self.binary_path);
        version_command_spec.add_argument("-v");

        build_command_spec
            .add_argument("--")
            .add_argument(version_command_spec.executable)
            .add_arguments(version_command_spec.arguments);

        Some(BuildInstructions {
            command_spec: build_command_spec,
            timeout: self.build_timeout,
        })
    }

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec = Self::bundled_command_spec(&self.binary_path);
        wrapped_spec
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build");
        self.apply_current_settings(&mut wrapped_spec);
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

    fn apply_current_settings(&self, command_spec: &mut CommandSpec) {
        command_spec
            .add_argument("--robot")
            .add_argument(&self.robot_yaml_path)
            // According to the `rcc --help`:
            // --controller string   internal, DO NOT USE (unless you know what you are doing)
            //
            // According to https://github.com/robocorp/rcc/blob/v16.5.0/docs/recipes.md#how-to-control-holotree-environments
            // This is one of three ways to controller where holotree spaces are created [...] when
            // applications are calling rcc, they should have their own "controller" identity, so
            // that all spaces created for one application are groupped together by prefix of their
            // "space" identity name.
            //
            // According to https://github.com/robocorp/rcc/blob/v16.5.0/docs/vocabulary.md#controller
            // This is tool or context that is currently running rcc command.
            //
            // From the code we can see, that the controlle is included in UserAgent of HTTP
            // requests for and the journaling (for example).
            //
            // In sum, ignoring the `DO NOT USE` seems correct.
            .add_argument("--controller")
            .add_argument(&self.controller)
            .add_argument("--space")
            .add_argument(&self.space);
    }
}

#[derive(Debug, PartialEq)]
pub struct BuildInstructions {
    pub command_spec: CommandSpec,
    pub timeout: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcc_build_instructions() {
        let mut expected_command_spec = CommandSpec::new("/bin/rcc");
        expected_command_spec
            .add_argument("--bundled")
            .add_argument("task")
            .add_argument("script")
            .add_argument("--robot")
            .add_argument("/a/b/c/robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_plan")
            .add_argument("--")
            .add_argument("/bin/rcc")
            .add_argument("--bundled")
            .add_argument("-v");

        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                remote_origin: None,
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_plan"),
                build_timeout: 123,
            }
            .build_instructions()
            .unwrap(),
            BuildInstructions {
                command_spec: expected_command_spec,
                timeout: 123,
            }
        )
    }

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
            .add_argument("--bundled")
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build")
            .add_argument("--robot")
            .add_argument("C:\\some_synthetic_test\\robot.yaml")
            .add_argument("--controller")
            .add_argument("robotmk")
            .add_argument("--space")
            .add_argument("my_plan")
            .add_argument("--")
            .add_argument("C:\\x\\y\\z.exe")
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                remote_origin: None,
                robot_yaml_path: Utf8PathBuf::from("C:\\some_synthetic_test\\robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_plan"),
                build_timeout: 600,
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }
}
