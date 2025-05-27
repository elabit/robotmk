use super::ResultCode;
use crate::command_spec::CommandSpec;
use crate::results::BuildOutcome;
use crate::termination::Cancelled;

#[derive(Clone, Debug, PartialEq)]
pub struct SystemEnvironment {}

impl SystemEnvironment {
    pub fn build(&self) -> Result<BuildOutcome, Cancelled> {
        Ok(BuildOutcome::NotNeeded)
    }

    pub fn wrap(&self, command_spec: CommandSpec) -> CommandSpec {
        command_spec
    }

    pub fn create_result_code(&self, exit_code: i32) -> ResultCode {
        if exit_code == 0 {
            return ResultCode::Success;
        }
        ResultCode::WrappedCommandFailed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap() {
        let mut to_be_wrapped = CommandSpec::new("C:\\x\\y\\z.exe");
        to_be_wrapped
            .add_argument("arg1")
            .add_argument("--flag")
            .add_argument("--option")
            .add_argument("option_value");
        to_be_wrapped
            .add_plain_env("PLAIN_KEY", "PLAIN_VALUE")
            .add_obfuscated_env("OBFUSCATED_KEY", "OBFUSCATED_VALUE");

        assert_eq!(
            SystemEnvironment {}.wrap(to_be_wrapped.clone()),
            to_be_wrapped
        );
    }
}
