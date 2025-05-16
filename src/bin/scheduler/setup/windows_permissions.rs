#![cfg(windows)]
use anyhow::{Context, bail};
use camino::Utf8Path;
use std::process::Command;

pub fn run_icacls_command<'a>(arguments: impl IntoIterator<Item = &'a str>) -> anyhow::Result<()> {
    run_command("icacls.exe", arguments)
}

pub fn grant_full_access(sid: &str, target_path: &Utf8Path) -> anyhow::Result<()> {
    let arguments = [
        target_path.as_ref(),
        "/grant",
        &format!("{sid}:(OI)(CI)F"),
        "/T",
    ];
    run_icacls_command(arguments).map_err(|e| {
        let message = format!("Adjusting permissions of {target_path} for SID `{sid}` failed");
        e.context(message)
    })
}

pub fn reset_access(target_path: &Utf8Path) -> anyhow::Result<()> {
    let arguments = [target_path.as_ref(), "/reset", "/T"];
    run_icacls_command(arguments).map_err(|e| {
        let message = format!("Resetting permissions of {target_path} failed");
        e.context(message)
    })
}

pub fn transfer_directory_ownership_to_admin_group_recursive(
    target_path: &Utf8Path,
) -> anyhow::Result<()> {
    run_takeown_command(["/a", "/r", "/f", target_path.as_str()]).map_err(|e| {
        e.context(format!(
            "Transfering ownership of {target_path} to administrator group failed (recursive)"
        ))
    })
}

fn run_command<'a>(
    program: &'a str,
    arguments: impl IntoIterator<Item = &'a str>,
) -> anyhow::Result<()> {
    let mut command = Command::new(program);
    command.args(arguments);
    let output = command
        .output()
        .context(format!("Calling {program} failed. Command:\n{command:?}"))?;
    if !output.status.success() {
        bail!(
            "{program} exited non-successfully.\n\nCommand:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    Ok(())
}

fn run_takeown_command<'a>(arguments: impl IntoIterator<Item = &'a str>) -> anyhow::Result<()> {
    run_command("takeown.exe", arguments)
}
