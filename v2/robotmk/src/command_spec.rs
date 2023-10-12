use std::convert::From;
use std::fmt::{Display, Formatter, Result};
use std::process::Command;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CommandSpec {
    pub executable: String,
    pub arguments: Vec<String>,
}

impl Display for CommandSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}", Command::from(self),)
    }
}

impl From<&CommandSpec> for Command {
    fn from(command_spec: &CommandSpec) -> Self {
        let mut command = Self::new(&command_spec.executable);
        command.args(&command_spec.arguments);
        command
    }
}

impl CommandSpec {
    pub fn new(executable: impl AsRef<str>) -> Self {
        Self {
            executable: executable.as_ref().into(),
            arguments: vec![],
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
        };
        assert_eq!(
            format!("{command_spec}"),
            "\"/my/binary\" \"mandatory\" \"--flag\" \"--option\" \"value\""
        );
    }

    #[test]
    fn command_from_command_spec() {
        let mut expected = Command::new("/my/binary");
        expected
            .arg("mandatory")
            .arg("--flag")
            .arg("--option")
            .arg("value");
        assert_eq!(
            format!(
                "{:?}",
                Command::from(&CommandSpec {
                    executable: String::from("/my/binary"),
                    arguments: vec![
                        String::from("mandatory"),
                        String::from("--flag"),
                        String::from("--option"),
                        String::from("value"),
                    ],
                })
            ),
            format!("{:?}", expected)
        )
    }

    #[test]
    fn new() {
        assert_eq!(
            CommandSpec::new("/my/binary"),
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec![],
            }
        )
    }

    #[test]
    fn add_argument() {
        let mut command_spec = CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![],
        };
        command_spec.add_argument("arg");
        assert_eq!(
            command_spec,
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec!["arg".into()],
            }
        );
    }

    #[test]
    fn add_arguments() {
        let mut command_spec = CommandSpec {
            executable: String::from("/my/binary"),
            arguments: vec![],
        };
        command_spec.add_arguments(vec!["arg1", "arg2"]);
        assert_eq!(
            command_spec,
            CommandSpec {
                executable: String::from("/my/binary"),
                arguments: vec!["arg1".into(), "arg2".into()],
            }
        );
    }
}
