use super::session::RunOutcome;
use crate::command_spec::CommandSpec;
use crate::logging::log_and_return_error;
use crate::termination::kill_process_tree;
use robotmk::termination::TerminationFlag;

use anyhow::{bail, Context, Result};
use async_std::{future::timeout, task::sleep as async_sleep};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{Duration as ChronoDuration, Local};
use futures::executor;
use log::{debug, error};
use std::fs::{read_to_string, write};
use std::process::Command;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use sysinfo::Pid;

pub fn run_task(task_spec: &TaskSpec) -> Result<RunOutcome> {
    debug!(
        "Running the following command as task {} for user {}:\n{}\n\nBase path: {}",
        task_spec.task_name, task_spec.user_name, task_spec.command_spec, task_spec.base_path
    );

    let paths = Paths::from(task_spec.base_path);
    create_task(task_spec, &paths)
        .context(format!("Failed to create task {}", task_spec.task_name))?;

    debug!("Starting task {}", task_spec.task_name);
    run_schtasks(["/run", "/tn", task_spec.task_name])
        .context(format!("Failed to start task {}", task_spec.task_name))?;

    if let Some(run_outcome) = match executor::block_on(timeout(
        Duration::from_secs(task_spec.timeout),
        wait_for_task_exit(task_spec.task_name, task_spec.termination_flag, &paths.pid),
    )) {
        Ok(task_wait_result) => task_wait_result.map_err(|err| {
            kill_and_delete_task(task_spec.task_name, &paths.pid);
            err
        })?,
        _ => {
            error!("Timed out");
            kill_and_delete_task(task_spec.task_name, &paths.pid);
            return Ok(RunOutcome::TimedOut);
        }
    } {
        return Ok(run_outcome);
    };
    debug!("Task {} completed", task_spec.task_name);

    delete_task(task_spec.task_name);

    let raw_exit_code = read_until_first_whitespace(&paths.exit_code)?;
    Ok(RunOutcome::Exited(Some(
        raw_exit_code
            .parse::<i32>()
            .context(format!("Failed to parse {} as i32", raw_exit_code))?,
    )))
}

pub struct TaskSpec<'a> {
    pub task_name: &'a str,
    pub command_spec: &'a CommandSpec,
    pub user_name: &'a str,
    pub base_path: &'a Utf8Path,
    pub timeout: u64,
    pub termination_flag: &'a TerminationFlag,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
struct Paths {
    script: Utf8PathBuf,
    stdout: Utf8PathBuf,
    stderr: Utf8PathBuf,
    pid: Utf8PathBuf,
    exit_code: Utf8PathBuf,
}

impl From<&Utf8Path> for Paths {
    fn from(base_path: &Utf8Path) -> Self {
        Self {
            // .bat is important here, otherwise, the Windows task scheduler won't know how to
            // execute this file.
            script: Utf8PathBuf::from(format!("{base_path}.bat")),
            stdout: Utf8PathBuf::from(format!("{base_path}.stdout")),
            stderr: Utf8PathBuf::from(format!("{base_path}.stderr")),
            pid: Utf8PathBuf::from(format!("{base_path}.pid")),
            exit_code: Utf8PathBuf::from(format!("{base_path}.exit_code")),
        }
    }
}

fn create_task(task_spec: &TaskSpec, paths: &Paths) -> Result<()> {
    write(
        &paths.script,
        build_task_script(task_spec.command_spec, paths),
    )
    .context(format!(
        "Failed to write script for task {} to {}",
        task_spec.task_name, paths.script
    ))?;
    debug!("Creating task {}", task_spec.task_name);
    let _ = run_schtasks(vec![
        "/create",
        "/tn",
        task_spec.task_name,
        "/tr",
        paths.script.as_str(),
        "/sc",
        "ONCE",
        "/ru",
        task_spec.user_name,
        "/it",
        "/rl",
        "LIMITED",
        "/st",
        // Since we are forced to provide this option, ensure that the task does not accidentally
        // start because we hit the start time.
        &Local::now()
            .checked_sub_signed(ChronoDuration::minutes(1))
            .context("Failed to compute value for start time option in task creation command")?
            .format("%H:%M")
            .to_string(),
        "/f",
    ])?;
    Ok(())
}

fn build_task_script(command_spec: &CommandSpec, paths: &Paths) -> String {
    [
        String::from("@echo off"),
        format!(
            "powershell.exe (Get-WmiObject Win32_Process -Filter ProcessId=$PID).ParentProcessId > {}",
            paths.pid
        ),
        format!("{command_spec} > {} 2> {}", paths.stdout, paths.stderr),
        format!("echo %errorlevel% > {}", paths.exit_code),
    ]
    .join("\n")
}

fn run_schtasks<T>(arguments: impl IntoIterator<Item = T>) -> Result<String>
where
    T: AsRef<str>,
{
    let mut command = Command::new("schtasks.exe");
    command.args(arguments.into_iter().map(|a| a.as_ref().to_string()));
    let output = command
        .output()
        .context(format!("Failed to run schtasks: {command:?}"))?;
    if !output.status.success() {
        bail!(format!(
            "schtasks exited non-successfully. Command:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
    String::from_utf8(output.stdout.clone()).context(format!(
        "Failed to decode stdout of schtasks. Command:\n{command:?}\n\nLossy stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    ))
}

async fn wait_for_task_exit(
    task_name: &str,
    termination_flag: &TerminationFlag,
    path_pid: &Utf8Path,
) -> Result<Option<RunOutcome>> {
    debug!("Waiting for task {} to complete", task_name);
    while query_if_task_is_running(task_name)
        .context(format!("Failed to query if task {task_name} is running"))?
    {
        if termination_flag.should_terminate() {
            kill_and_delete_task(task_name, path_pid);
            return Ok(Some(RunOutcome::Terminated));
        }
        async_sleep(Duration::from_millis(250)).await
    }
    Ok(None)
}

fn query_if_task_is_running(task_name: &str) -> Result<bool> {
    let schtasks_stdout = run_schtasks(["/query", "/tn", task_name, "/fo", "CSV", "/nh"])?;
    Ok(schtasks_stdout.contains("Running"))
}

fn kill_and_delete_task(task_name: &str, path_pid: &Utf8Path) {
    // schtasks.exe /end ... terminates the batch script, but child processes will survive ...
    error!("Killing and deleting task {task_name}");
    let _ = kill_task(path_pid).map_err(log_and_return_error);
    delete_task(task_name);
}

fn kill_task(path_pid: &Utf8Path) -> Result<()> {
    let raw_pid = read_pid(path_pid)?;
    kill_process_tree(
        &Pid::from_str(&raw_pid).context(format!("Failed to parse {} as PID", raw_pid))?,
    );
    Ok(())
}

fn delete_task(task_name: &str) {
    debug!("Deleting task {task_name}");
    let _ = run_schtasks(["/delete", "/tn", task_name, "/f"])
        .context(format!("Failed to delete task {}", task_name))
        .map_err(log_and_return_error);
}

fn read_until_first_whitespace(path: &Utf8Path) -> Result<String> {
    let content = read_to_string(path).context(format!("Failed to read {path}"))?;
    Ok(content
        .split_whitespace()
        .next()
        .context(format!("{path} is empty"))?
        .to_string())
}

fn read_pid(path: &Utf8Path) -> Result<String> {
    match read_until_first_whitespace(path) {
        Ok(pid) => return Ok(pid),
        Err(err) => {
            log_and_return_error(err.context(format!(
                "Failed to read PID from {path}, will sleep 1s and try one more time"
            )));
        }
    };
    sleep(Duration::from_secs(1));
    read_until_first_whitespace(path).context(format!("Failed to read PID from {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use tempfile::NamedTempFile;

    #[test]
    fn paths_from_base_path() {
        assert_eq!(
            Paths::from(Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0").as_ref()),
            Paths {
                script: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.bat"),
                stdout: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.stdout"),
                stderr: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.stderr"),
                pid: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.pid"),
                exit_code: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.exit_code"),
            }
        )
    }

    #[test]
    fn test_build_task_script() {
        let mut command_spec = CommandSpec::new("C:\\somewhere\\rcc.exe");
        command_spec
            .add_argument("mandatory")
            .add_argument("--some-flag")
            .add_argument("--some-option")
            .add_argument("some-value");
        assert_eq!(
            build_task_script(
                &command_spec,
                &Paths::from(Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0").as_ref())
            ),
            "@echo off
powershell.exe (Get-WmiObject Win32_Process -Filter ProcessId=$PID).ParentProcessId \
> C:\\working\\suites\\my_suite\\123\\0.pid
\"C:\\\\somewhere\\\\rcc.exe\" \"mandatory\" \"--some-flag\" \"--some-option\" \"some-value\" \
> C:\\working\\suites\\my_suite\\123\\0.stdout 2> C:\\working\\suites\\my_suite\\123\\0.stderr
echo %errorlevel% > C:\\working\\suites\\my_suite\\123\\0.exit_code"
        )
    }

    #[test]
    fn test_read_until_first_whitespace_ok() -> Result<()> {
        let temp_path = NamedTempFile::new()?.into_temp_path();
        write(&temp_path, "123\n456")?;
        assert_eq!(
            read_until_first_whitespace(&Utf8PathBuf::try_from(temp_path.to_path_buf())?)?,
            "123"
        );
        Ok(())
    }

    #[test]
    fn test_read_until_first_whitespace_empty() -> Result<()> {
        assert!(format!(
            "{:?}",
            read_until_first_whitespace(&Utf8PathBuf::try_from(
                NamedTempFile::new()?.into_temp_path().to_path_buf(),
            )?)
            .err()
            .unwrap()
        )
        .contains("is empty"));
        Ok(())
    }
}
