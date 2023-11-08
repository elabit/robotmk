use super::command_spec::CommandSpec;
use super::termination::kill_process_tree;
use robotmk::termination::TerminationFlag;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use log::{debug, error};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;
use sysinfo::{Pid, PidExt};
use tokio::time::{timeout, sleep};

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

async fn wait_for_child_exit(
    flag: &TerminationFlag,
    child: &mut Child,
) -> Result<ChildProcessOutcome> {
    loop {
        if flag.should_terminate() {
            kill_child_tree(child);
            return Ok(ChildProcessOutcome::Terminated);
        }

        match child.try_wait() {
            Ok(Some(exit_status)) => return Ok(ChildProcessOutcome::Exited(exit_status)),
            Ok(None) => {}
            e => {
                kill_child_tree(child);
                e.context(format!(
                    "Failed to query exit status of process {}, killing",
                    child.id()
                ))?;
            }
        }
        sleep(Duration::from_millis(250)).await
    }
}

#[tokio::main]
async fn wait_with_timeout(
    duration: Duration,
    flag: &TerminationFlag,
    child: &mut Child,
) -> Result<ChildProcessOutcome> {
    let child_future = wait_for_child_exit(flag, &mut child);
    match timeout(duration, child_future).await {
        Ok(child_result) => child_result,
        _ => {
            error!("Timed out");
            kill_child_tree(&child);
            Ok(ChildProcessOutcome::TimedOut)
        }
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

        let mut child = command.spawn().context("Failed to spawn subprocess")?;
        wait_with_timeout(Duration::from_secs(self.timeout), self.termination_flag, &mut child)
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

fn kill_child_tree(child: &Child) {
    kill_process_tree(&Pid::from_u32(child.id()))
}
