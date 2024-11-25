use std::convert::From;
use std::ffi::OsString;
use std::fmt::{Display, Formatter, Result};
use std::process::Command;

#[derive(Debug, PartialEq)]
pub struct CommandSpec {
    pub executable: String,
    pub arguments: Vec<String>,
    pub envs_rendered_obfuscated: Vec<(String, String)>,
}

impl Display for CommandSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let env_str = self
            .envs_rendered_obfuscated
            .iter()
            .map(|(k, _)| format!("{k}=***"))
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "{env_str} {}", self.to_command_string())
    }
}

impl From<&CommandSpec> for Command {
    fn from(command_spec: &CommandSpec) -> Self {
        let mut command = Self::new(&command_spec.executable);
        command.args(&command_spec.arguments);
        command.envs(
            command_spec
                .envs_rendered_obfuscated
                .iter()
                .map(|(k, v)| (OsString::from(&k), OsString::from(&v))),
        );
        command
    }
}

impl From<&CommandSpec> for tokio::process::Command {
    fn from(command_spec: &CommandSpec) -> Self {
        tokio::process::Command::from(Command::from(command_spec))
    }
}

impl CommandSpec {
    pub fn new(executable: impl AsRef<str>) -> Self {
        Self {
            executable: executable.as_ref().into(),
            arguments: vec![],
            envs_rendered_obfuscated: vec![],
        }
    }

    pub fn add_argument(&mut self, argument: impl AsRef<str>) -> &mut Self {
        self.arguments.push(argument.as_ref().into());
        self
    }

    pub fn add_arguments<T>(&mut self, arguments: impl IntoIterator<Item = T>) -> &mut Self
    where
        T: AsRef<str>,
    {
        self.arguments
            .extend(arguments.into_iter().map(|s| s.as_ref().into()));
        self
    }

    pub fn add_obfuscated_env(&mut self, key: String, value: String) -> &mut Self {
        self.envs_rendered_obfuscated.push((key, value));
        self
    }

    pub fn to_command_string(&self) -> String {
        let mut command = Command::new(self.executable.clone());
        command.args(&self.arguments);
        format!("{:?}", command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        let command_spec = CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![
                String::from("mandatory"),
                String::from("--flag"),
                String::from("--option"),
                String::from("value"),
            ],
            envs_rendered_obfuscated: vec![("RCC_REMOTE_ORIGIN".into(), "http://1.com".into())],
        };
        let expected =
            "RCC_REMOTE_ORIGIN=*** \"/my/binary\" \"mandatory\" \"--flag\" \"--option\" \"value\"";
        assert_eq!(format!("{command_spec}"), expected);
    }

    #[test]
    fn command_from_command_spec() {
        let mut expected = Command::new("/my/binary");
        expected
            .arg("mandatory")
            .arg("--flag")
            .arg("--option")
            .arg("value")
            .env("obfuscated_key", "obfuscated_val");
        let command = Command::from(&CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![
                String::from("mandatory"),
                String::from("--flag"),
                String::from("--option"),
                String::from("value"),
            ],
            envs_rendered_obfuscated: vec![(
                String::from("obfuscated_key"),
                String::from("obfuscated_val"),
            )],
        });
        assert_eq!(command.get_program(), expected.get_program());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            expected.get_args().collect::<Vec<_>>()
        );
        assert_eq!(
            command.get_envs().collect::<Vec<_>>(),
            expected.get_envs().collect::<Vec<_>>()
        );
    }

    #[test]
    fn new() {
        assert_eq!(
            CommandSpec::new("/my/binary"),
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec![],
                envs_rendered_obfuscated: vec![],
            }
        )
    }

    #[test]
    fn add_argument() {
        let mut command_spec = CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![],
            envs_rendered_obfuscated: vec![],
        };
        command_spec.add_argument("arg");
        assert_eq!(
            command_spec,
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec!["arg".into()],
                envs_rendered_obfuscated: vec![],
            }
        );
    }

    #[test]
    fn add_obfuscated_env() {
        let mut command_spec = CommandSpec::new("/my/binary");
        command_spec.add_obfuscated_env("key".to_string(), "val".to_string());
        assert_eq!(
            command_spec.envs_rendered_obfuscated,
            [(String::from("key"), String::from("val"))]
        );
    }

    #[test]
    fn add_arguments() {
        let mut command_spec = CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![],
            envs_rendered_obfuscated: vec![],
        };
        command_spec.add_arguments(vec!["arg1", "arg2"]);
        assert_eq!(
            command_spec,
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec!["arg1".into(), "arg2".into()],
                envs_rendered_obfuscated: vec![],
            }
        );
    }
}
