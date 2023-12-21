use anyhow::{bail, Context, Result as AnyhowResult};
use std::process::Command;

pub fn run_icacls_command<'a>(arguments: impl IntoIterator<Item = &'a str>) -> AnyhowResult<()> {
    let mut command = Command::new("icacls.exe");
    command.args(arguments);
    let output = command
        .output()
        .context(format!("Calling icacls.exe failed. Command:\n{command:?}"))?;
    if !output.status.success() {
        bail!(
            "icacls.exe exited non-successfully.\n\nCommand:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    Ok(())
}
