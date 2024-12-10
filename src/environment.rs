use crate::command_spec::CommandSpec;
use crate::config::EnvironmentConfig;

use camino::{Utf8Path, Utf8PathBuf};

pub enum ResultCode {
    AllTestsPassed,
    RobotCommandFailed,
    EnvironmentFailed,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
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
    pub catalog_zip: Option<Utf8PathBuf>,
    pub robot_yaml_path: Utf8PathBuf,
    pub controller: String,
    pub space: String,
    pub build_timeout: u64,
    pub build_runtime_directory: Utf8PathBuf,
    pub robocorp_home: String,
}

impl Environment {
    pub fn new(
        base_dir: &Utf8Path,
        robocorp_home: &Utf8Path,
        plan_id: &str,
        rcc_binary_path: &Utf8Path,
        environment_config: &EnvironmentConfig,
        build_runtime_directory: &Utf8Path,
    ) -> Self {
        match environment_config {
            EnvironmentConfig::System => Self::System(SystemEnvironment {}),
            EnvironmentConfig::Rcc(rcc_environment_config) => Self::Rcc(RCCEnvironment {
                binary_path: rcc_binary_path.to_path_buf(),
                remote_origin: rcc_environment_config.remote_origin.clone(),
                catalog_zip: rcc_environment_config.catalog_zip.clone(),
                robot_yaml_path: base_dir.join(&rcc_environment_config.robot_yaml_path),
                controller: String::from("robotmk"),
                space: plan_id.to_string(),
                build_timeout: rcc_environment_config.build_timeout,
                build_runtime_directory: build_runtime_directory.to_path_buf(),
                robocorp_home: robocorp_home.to_string(),
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
    pub fn bundled_command_spec(binary_path: &Utf8Path, robocorp_home: String) -> CommandSpec {
        let mut command_spec = CommandSpec::new(binary_path);
        command_spec.add_argument("--bundled");
        command_spec.add_plain_env("ROBOCORP_HOME", &robocorp_home);
        command_spec
    }

    pub fn build_instructions(&self) -> Option<BuildInstructions> {
        let import_command_spec = self.catalog_zip.as_ref().map(|zip| {
            let mut spec =
                Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
            spec.add_argument("holotree")
                .add_argument("import")
                .add_argument(zip);
            spec
        });

        let mut build_command_spec =
            Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
        build_command_spec
            .add_argument("task")
            .add_argument("script");
        self.apply_current_settings(&mut build_command_spec);
        if let Some(remote_origin) = &self.remote_origin {
            build_command_spec.add_obfuscated_env("RCC_REMOTE_ORIGIN", remote_origin);
        }
        build_command_spec.add_argument("--").add_argument(
            #[cfg(unix)]
            "true",
            #[cfg(windows)]
            "cmd.exe",
        );

        Some(BuildInstructions {
            import_command_spec,
            build_command_spec,
            timeout: self.build_timeout,
            runtime_directory: self.build_runtime_directory.clone(),
        })
    }

    fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        let mut wrapped_spec =
            Self::bundled_command_spec(&self.binary_path, self.robocorp_home.clone());
        wrapped_spec
            .add_argument("task")
            .add_argument("script")
            .add_argument("--no-build");
        self.apply_current_settings(&mut wrapped_spec);
        wrapped_spec
            .add_argument("--")
            .add_argument(command_spec.executable)
            .add_arguments(command_spec.arguments);
        for (key, value) in command_spec.envs_rendered_plain {
            wrapped_spec.add_plain_env(key, value);
        }
        for (key, value) in command_spec.envs_rendered_obfuscated {
            wrapped_spec.add_obfuscated_env(key, value);
        }
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
    pub import_command_spec: Option<CommandSpec>,
    pub build_command_spec: CommandSpec,
    pub timeout: u64,
    pub runtime_directory: Utf8PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcc_build_instructions() {
        let mut expected_import_command_spec = CommandSpec::new("/bin/rcc");
        expected_import_command_spec
            .add_argument("--bundled")
            .add_argument("holotree")
            .add_argument("import")
            .add_argument("/catalog.zip")
            .add_plain_env("ROBOCORP_HOME", "~/.robocorp/");

        let mut expected_build_command_spec = CommandSpec::new("/bin/rcc");
        expected_build_command_spec
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
            .add_argument(
                #[cfg(unix)]
                "true",
                #[cfg(windows)]
                "cmd.exe",
            )
            .add_plain_env("ROBOCORP_HOME", "~/.robocorp/")
            .add_obfuscated_env("RCC_REMOTE_ORIGIN", "http://1.com");

        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("/bin/rcc"),
                remote_origin: Some("http://1.com".into()),
                catalog_zip: Some("/catalog.zip".into()),
                robot_yaml_path: Utf8PathBuf::from("/a/b/c/robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_plan"),
                build_timeout: 123,
                build_runtime_directory: Utf8PathBuf::from("/runtime"),
                robocorp_home: String::from("~/.robocorp/"),
            }
            .build_instructions()
            .unwrap(),
            BuildInstructions {
                import_command_spec: Some(expected_import_command_spec),
                build_command_spec: expected_build_command_spec,
                timeout: 123,
                runtime_directory: Utf8PathBuf::from("/runtime")
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
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");
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
            .add_argument("option_value")
            .add_plain_env("ROBOCORP_HOME", "~/.robocorp/")
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");
        assert_eq!(
            RCCEnvironment {
                binary_path: Utf8PathBuf::from("C:\\bin\\z.exe"),
                remote_origin: None,
                catalog_zip: None,
                robot_yaml_path: Utf8PathBuf::from("C:\\some_synthetic_test\\robot.yaml"),
                controller: String::from("robotmk"),
                space: String::from("my_plan"),
                build_timeout: 600,
                build_runtime_directory: Utf8PathBuf::default(),
                robocorp_home: String::from("~/.robocorp/"),
            }
            .wrap(command_spec_for_wrap()),
            expected
        );
    }
}
