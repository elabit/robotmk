use super::command_spec::CommandSpec;
use super::termination::TerminationFlag;
use anyhow::{bail, Context, Result};
use async_std::{future::timeout, task::sleep};
use camino::Utf8PathBuf;
use futures::executor;
use log::{debug, error};
use std::collections::{HashMap, HashSet};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;
use sysinfo::{Pid, PidExt, Process, ProcessExt, System, SystemExt};

pub struct ChildProcessSupervisor<'a> {
    pub command_spec: CommandSpec,
    pub stdio_paths: Option<StdioPaths>,
    pub timeout: u64,
    pub termination_flag: &'a TerminationFlag,
}

pub struct StdioPaths {
    pub stdout: Utf8PathBuf,
    pub stderr: Utf8PathBuf,
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
        match executor::block_on(timeout(
            Duration::from_secs(self.timeout),
            self.wait_for_child_exit(&mut child),
        )) {
            Ok(child_result) => child_result,
            _ => {
                error!("Timed out");
                kill_process_tree(&mut child);
                Ok(ChildProcessOutcome::TimedOut)
            }
        }
    }

    fn build_command(&self) -> Result<Command> {
        let mut command = Command::from(&self.command_spec);
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

    async fn wait_for_child_exit(&self, child: &mut Child) -> Result<ChildProcessOutcome> {
        loop {
            if self.termination_flag.should_terminate() {
                kill_process_tree(child);
                bail!("Terminated")
            }

            if let Some(exit_status) = child
                .try_wait()
                .context(format!(
                    "Failed to query exit status of process {}, killing",
                    child.id()
                ))
                .map_err(|err| {
                    kill_process_tree(child);
                    err
                })?
            {
                return Ok(ChildProcessOutcome::Exited(exit_status));
            }

            sleep(Duration::from_millis(250)).await
        }
    }
}

pub enum ChildProcessOutcome {
    Exited(ExitStatus),
    TimedOut,
}

// This is a non-cooperative termination (SIGKILL) of the entire child process tree. What we would
// actually like to do is to shut down our child co-operatively and leave the termination of any
// non-direct children further down the tree to our child. However, Windows offers no API for this
// (there is no SIGTERM on Windows), so we instead kill the entire tree.
fn kill_process_tree(child: &mut Child) {
    let mut system = System::new_all();
    system.refresh_processes();
    let _ = child.kill();
    kill_all_children(&Pid::from_u32(child.id()), system.processes());
}

fn kill_all_children<'a>(top_pid: &'a Pid, processes: &'a HashMap<Pid, Process>) {
    let mut pids_in_tree = HashSet::from([top_pid]);

    loop {
        let current_tree_size = pids_in_tree.len();
        add_and_kill_direct_children(&mut pids_in_tree, processes);
        if pids_in_tree.len() == current_tree_size {
            break;
        }
    }
}

fn add_and_kill_direct_children<'a>(
    pids_in_tree: &mut HashSet<&'a Pid>,
    processes: &'a HashMap<Pid, Process>,
) {
    for (pid, parent_pid, process) in processes.iter().filter_map(|(pid, process)| {
        process
            .parent()
            .map(|parent_pid| (pid, parent_pid, process))
    }) {
        {
            if pids_in_tree.contains(&parent_pid) {
                pids_in_tree.insert(pid);
                process.kill();
            }
        }
    }
}
