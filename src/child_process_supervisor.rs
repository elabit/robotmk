use crate::command_spec::CommandSpec;
use crate::termination::{kill_process_tree, waited, Outcome};

use anyhow::{Context, Result as AnyhowResult};
use camino::Utf8PathBuf;
use log::debug;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use sysinfo::Pid;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub struct ChildProcessSupervisor<'a> {
    pub command_spec: &'a CommandSpec,
    pub stdio_paths: Option<StdioPaths>,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
}

pub struct StdioPaths {
    pub stdout: Utf8PathBuf,
    pub stderr: Utf8PathBuf,
}

impl ChildProcessSupervisor<'_> {
    pub fn run(&self) -> AnyhowResult<Outcome<AnyhowResult<ExitStatus>>> {
        let mut command: Command = self.build_command()?;

        let (stdout_path, stderr_path) = if let Some(stdio_paths) = &self.stdio_paths {
            (
                stdio_paths.stdout.to_string(),
                stdio_paths.stderr.to_string(),
            )
        } else {
            ("inherited".into(), "inherited".into())
        };
        debug!(
            "Executing: {}, Stdout: {stdout_path}, Stderr: {stderr_path}",
            self.command_spec,
        );

        wait_for_child(
            Duration::from_secs(self.timeout),
            self.cancellation_token,
            &mut command,
        )
    }

    fn build_command(&self) -> AnyhowResult<Command> {
        let mut command = Command::from(self.command_spec);
        if let Some(stdio_paths) = &self.stdio_paths {
            command
                .stdout(std::fs::File::create(&stdio_paths.stdout).context(format!(
                    "Failed to open {} for stdout capturing",
                    stdio_paths.stdout
                ))?)
                .stderr(std::fs::File::create(&stdio_paths.stderr).context(format!(
                    "Failed to open {} for stderr capturing",
                    stdio_paths.stderr
                ))?);
        } else {
            command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }
        Ok(command)
    }
}

#[tokio::main]
async fn wait_for_child(
    duration: Duration,
    flag: &CancellationToken,
    command: &mut Command,
) -> AnyhowResult<Outcome<AnyhowResult<ExitStatus>>> {
    let child = &mut command.spawn().context("Failed to spawn subprocess")?;
    match waited(duration, flag, child.wait()).await {
        Outcome::Timeout => {
            kill_child_tree(child);
            Ok(Outcome::Timeout)
        }
        Outcome::Cancel => {
            kill_child_tree(child);
            Ok(Outcome::Cancel)
        }
        Outcome::Completed(result) => {
            if result.is_err() {
                kill_child_tree(child);
            }
            Ok(Outcome::Completed(
                result.context("Failed to retrieve exit status of subprocess"),
            ))
        }
    }
}

fn kill_child_tree(child: &tokio::process::Child) {
    if let Some(id) = child.id() {
        kill_process_tree(&Pid::from_u32(id))
    }
}
