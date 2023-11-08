use super::command_spec::CommandSpec;
use super::termination::kill_process_tree;
use robotmk::termination::{waited, Outcome, TerminationFlag};

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use log::{debug, error};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use sysinfo::{Pid, PidExt};
use tokio::process::{Child, Command};

pub struct ChildProcessSupervisor<'a> {
    pub command_spec: &'a CommandSpec,
    pub stdio_paths: Option<StdioPaths>,
    pub timeout: u64,
    pub termination_flag: &'a TerminationFlag,
}

pub struct StdioPaths {
    pub stdout: Utf8PathBuf,
    pub stderr: Utf8PathBuf,
}

fn wait_for_child(
    duration: Duration,
    flag: &TerminationFlag,
    child: &mut Child,
) -> Result<ChildProcessOutcome> {
    match waited(duration, flag, child.wait()) {
        Outcome::Timeout => {
            error!("Timed out");
            kill_child_tree(child);
            Ok(ChildProcessOutcome::TimedOut)
        }
        Outcome::Cancel => {
            kill_child_tree(child);
            Ok(ChildProcessOutcome::Terminated)
        }
        Outcome::Completed(Err(e)) => {
            kill_child_tree(child);
            Err(e.into())
        }
        Outcome::Completed(Ok(o)) => Ok(ChildProcessOutcome::Exited(o)),
    }
}

impl ChildProcessSupervisor<'_> {
    pub fn run(&self) -> Result<ChildProcessOutcome> {
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
            self.termination_flag,
            &mut command.spawn().context("Failed to spawn subprocess")?,
        )
    }

    fn build_command(&self) -> Result<Command> {
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

pub enum ChildProcessOutcome {
    Exited(ExitStatus),
    TimedOut,
    Terminated,
}

fn kill_child_tree(child: &tokio::process::Child) {
    if let Some(id) = child.id() {
        kill_process_tree(&Pid::from_u32(id))
    }
}
