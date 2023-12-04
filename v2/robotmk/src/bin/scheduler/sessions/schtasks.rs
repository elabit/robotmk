use super::session::RunOutcome;
use crate::command_spec::CommandSpec;
use crate::logging::log_and_return_error;
use crate::termination::kill_process_tree;
use robotmk::termination::{waited, Outcome};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{Duration as ChronoDuration, Local};
use log::{debug, error, warn};
use std::fs::{read_to_string, remove_file, write};
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use sysinfo::Pid;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;

fn wait_for_task_exit(task: &TaskSpec, paths: &Paths) -> Result<RunOutcome> {
    let duration = Duration::from_secs(task.timeout);
    let queried = query(task.task_name, &paths.exit_code);
    match waited(duration, task.cancellation_token, queried) {
        Outcome::Cancel => {
            kill_and_delete_task(task.task_name, paths);
            Ok(RunOutcome::TimedOut)
        }
        Outcome::Timeout => {
            error!("Timeout");
            kill_and_delete_task(task.task_name, paths);
            Ok(RunOutcome::Terminated)
        }
        Outcome::Completed(Err(e)) => {
            kill_and_delete_task(task.task_name, paths);
            Err(e)
        }
        Outcome::Completed(Ok(code)) => {
            debug!("Task {} completed", task.task_name);
            delete_task(task.task_name);
            Ok(RunOutcome::Exited(Some(code)))
        }
    }
}

pub fn run_task(task_spec: &TaskSpec) -> Result<RunOutcome> {
    debug!(
        "Running the following command as task {} for user {}:\n{}\n\nBase path: {}",
        task_spec.task_name, task_spec.user_name, task_spec.command_spec, task_spec.base_path
    );
    assert_session_is_active(task_spec.user_name)?;

    let paths = Paths::from(task_spec.base_path);
    create_task(task_spec, &paths)
        .context(format!("Failed to create task {}", task_spec.task_name))?;
    start_task(task_spec.task_name, &paths.run_flag)?;

    wait_for_task_exit(task_spec, &paths)
}

pub struct TaskSpec<'a> {
    pub task_name: &'a str,
    pub command_spec: &'a CommandSpec,
    pub user_name: &'a str,
    pub base_path: &'a Utf8Path,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
struct Paths {
    script: Utf8PathBuf,
    run_flag: Utf8PathBuf,
    pid: Utf8PathBuf,
    stdout: Utf8PathBuf,
    stderr: Utf8PathBuf,
    exit_code: Utf8PathBuf,
}

impl From<&Utf8Path> for Paths {
    fn from(base_path: &Utf8Path) -> Self {
        Self {
            // .bat is important here, otherwise, the Windows task scheduler won't know how to
            // execute this file.
            script: Utf8PathBuf::from(format!("{base_path}.bat")),
            run_flag: Utf8PathBuf::from(format!("{base_path}.run_flag")),
            pid: Utf8PathBuf::from(format!("{base_path}.pid")),
            stdout: Utf8PathBuf::from(format!("{base_path}.stdout")),
            stderr: Utf8PathBuf::from(format!("{base_path}.stderr")),
            exit_code: Utf8PathBuf::from(format!("{base_path}.exit_code")),
        }
    }
}

fn assert_session_is_active(user_name: &str) -> Result<()> {
    let mut query_user_command = Command::new("query");
    query_user_command.arg("user");
    if check_if_user_has_active_session(
        user_name,
        &String::from_utf8_lossy(
            &query_user_command
                .output()
                .context(format!(
                    "Failed to query if user {user_name} has an active session"
                ))?
                .stdout,
        ),
    ) {
        return Ok(());
    }
    bail!("No active session for user {user_name} found")
}

fn check_if_user_has_active_session(user_name: &str, query_user_stdout: &str) -> bool {
    for line in query_user_stdout.lines().skip(1) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(user_name_of_session) = words.first() else {
            continue;
        };
        let Some(session_state) = words.get(3) else {
            continue;
        };
        if (&user_name == user_name_of_session
            || &format!(
                // `>` marks the current session
                ">{user_name}"
            ) == user_name_of_session)
            && session_state == &"Active"
        {
            return true;
        }
    }
    false
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
        // `schtasks.exe /end ...` seems utterly useless. Hence, we employ this run flag to signal
        // our task to terminate (in addition to killing the process if we were able to read the
        // PID, which is not the case if the task has just started).
        format!("if not exist {} exit /b 1", paths.run_flag),
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

fn start_task(task_name: &str, path_run_flag: &Utf8Path) -> Result<()> {
    debug!("Starting task {task_name}");
    write(path_run_flag, "").context(format!("Failed to create run flag file {path_run_flag}"))?;
    run_schtasks(["/run", "/tn", task_name])
        .context(format!("Failed to start task {task_name}"))?;
    Ok(())
}

async fn query(task_name: &str, exit_path: &Utf8Path) -> Result<i32> {
    debug!("Waiting for task {} to complete", task_name);
    while query_if_task_is_running(task_name)
        .context(format!("Failed to query if task {task_name} is running"))?
    {
        yield_now().await
    }

    let raw_exit_code = read_until_first_whitespace(exit_path)?;
    let exit_code: i32 = raw_exit_code
        .parse()
        .context(format!("Failed to parse {} as i32", raw_exit_code))?;
    Ok(exit_code)
}

fn query_if_task_is_running(task_name: &str) -> Result<bool> {
    let schtasks_stdout = run_schtasks(["/query", "/tn", task_name, "/fo", "CSV", "/nh"])?;
    Ok(schtasks_stdout.contains("Running"))
}

fn kill_and_delete_task(task_name: &str, paths: &Paths) {
    error!("Killing and deleting task {task_name}");
    kill_task(paths);
    delete_task(task_name);
}

fn kill_task(paths: &Paths) {
    let _ = remove_file(&paths.run_flag)
        .context(format!("Failed to remove {}", paths.run_flag))
        .map_err(log_and_return_error);
    let _ = kill_task_via_pid(&paths.pid).map_err(|error| {
        warn!("{:?}", error);
        error
    });
}

fn kill_task_via_pid(path_pid: &Utf8Path) -> Result<()> {
    let raw_pid = read_until_first_whitespace(path_pid)
        .context(format!("Failed to read PID from {path_pid}"))?;
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
                run_flag: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.run_flag"),
                pid: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.pid"),
                stdout: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.stdout"),
                stderr: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.stderr"),
                exit_code: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.exit_code"),
            }
        )
    }

    #[test]
    fn check_if_user_has_active_session_ok() {
        assert!(check_if_user_has_active_session(
            "vagrant",
            " USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
>vagrant               console             1  Active      none   12/4/2023 9:35 AM
 vagrant2              rdp-tcp#0           2  Active          .  12/4/2023 9:36 AM"
        ))
    }

    #[test]
    fn check_if_user_has_active_session_disconnected() {
        assert!(!check_if_user_has_active_session(
            "vagrant2",
            " USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
>vagrant               console             1  Active      none   12/4/2023 9:35 AM
 vagrant2                                  2  Disc            .  12/4/2023 9:36 AM"
        ))
    }

    #[test]
    fn check_if_user_has_active_session_no_session() {
        assert!(!check_if_user_has_active_session(
            "vagrant2",
            " USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
>vagrant               console             1  Active      none   12/4/2023 9:35 AM"
        ))
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
if not exist C:\\working\\suites\\my_suite\\123\\0.run_flag exit /b 1
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
