use crate::command_spec::CommandSpec;
use anyhow::Context;
use camino::Utf8Path;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RobotmkEnvironmentManifest {
    #[serde(default)]
    pub post_build_commands: Vec<PostBuildCommand>,
}

#[derive(Deserialize)]
pub struct PostBuildCommand {
    pub name: String,
    pub command: Vec<String>,
}

impl From<&PostBuildCommand> for Option<CommandSpec> {
    fn from(value: &PostBuildCommand) -> Self {
        if value.command.is_empty() {
            return None;
        }
        let mut command_spec = CommandSpec::new(&value.command[0]);
        command_spec.add_arguments(&value.command[1..]);
        Some(command_spec)
    }
}

pub fn parse_robotmk_environment_manifest(
    path: &Utf8Path,
) -> anyhow::Result<RobotmkEnvironmentManifest> {
    serde_yaml::from_str::<RobotmkEnvironmentManifest>(&std::fs::read_to_string(path).context(
        format!("Failed to read Robotmk environment manifest file {path}",),
    )?)
    .context("Failed to parse Robotmk environment manifest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_spec_from_post_build_command_executable_only() {
        let command_spec: Option<CommandSpec> = (&PostBuildCommand {
            name: "test".into(),
            command: vec!["echo".into()],
        })
            .into();
        assert_eq!(
            command_spec.unwrap(),
            CommandSpec {
                executable: "echo".into(),
                arguments: vec![],
                envs_rendered_plain: vec![],
                envs_rendered_obfuscated: vec![],
            }
        );
    }

    #[test]
    fn command_spec_from_post_build_command_with_args() {
        let command_spec: Option<CommandSpec> = (&PostBuildCommand {
            name: "test".into(),
            command: vec!["exec".into(), "--arg".into(), "value".into()],
        })
            .into();
        assert_eq!(
            command_spec.unwrap(),
            CommandSpec {
                executable: "exec".into(),
                arguments: vec!["--arg".into(), "value".into()],
                envs_rendered_plain: vec![],
                envs_rendered_obfuscated: vec![],
            }
        );
    }

    #[test]
    fn command_spec_from_post_build_command_empty() {
        let command_spec: Option<CommandSpec> = (&PostBuildCommand {
            name: "test".into(),
            command: vec![],
        })
            .into();
        assert!(command_spec.is_none());
    }
}
