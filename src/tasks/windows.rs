use crate::command_spec::CommandSpec;
use crate::tasks::TaskSpec;
use crate::termination::{kill_process_tree, waited, Outcome};

use anyhow::{bail, Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use std::fs;
use std::time::Duration;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;
use windows::core::{ComInterface, Result as WinApiResult, BSTR};
use windows::Win32::Foundation::VARIANT_FALSE;
use windows::Win32::System::{Com, TaskScheduler, Variant::VARIANT};

pub fn run_task(task_spec: &TaskSpec) -> AnyhowResult<Outcome<i32>> {
    debug!(
        "Running the following command as task {} for user {}:\n{}\n\nBase path: {}",
        task_spec.task_name, task_spec.user_name, task_spec.command_spec, task_spec.base_path
    );
    assert_session_is_present(task_spec.user_name)?;

    let paths = Paths::from(task_spec.base_path);
    let task_manager = TaskManager::new().context("Failed to create new TaskManager")?;

    fs::write(
        &paths.script,
        build_task_script(task_spec.task_name, task_spec.command_spec, &paths),
    )
    .context(format!(
        "Failed to write script for task {} to {}",
        task_spec.task_name, paths.script
    ))?;

    let task = task_manager
        .create_task(task_spec.task_name, &paths.script, task_spec.user_name)
        .context(format!("Failed to create task {}", task_spec.task_name))?;
    let outcome = task_manager
        .run_task(&task, task_spec.timeout, task_spec.cancellation_token)
        .context(format!("Failed to run task {}", task_spec.task_name))?;

    let _ = task_manager
        .delete_task(&task)
        .context(format!("Failed to delete task {}", task_spec.task_name))
        .map_err(|e| error!("{:?}", e));

    match outcome {
        Outcome::Cancel => return Ok(Outcome::Cancel),
        Outcome::Timeout => return Ok(Outcome::Timeout),
        Outcome::Completed(winapi_result) => winapi_result.context(format!(
            "Error while querying if task {} is still running",
            task_spec.task_name
        )),
    }?;

    Ok(Outcome::Completed(read_exit_code(&paths.exit_code)?))
}

fn assert_session_is_present(user_name: &str) -> AnyhowResult<()> {
    let mut query_user_command = std::process::Command::new("query");
    query_user_command.arg("user");
    if check_if_user_has_session(
        user_name,
        &String::from_utf8_lossy(
            &query_user_command
                .output()
                .context("Failed to query user sessions (`query user`)")?
                .stdout,
        ),
    ) {
        return Ok(());
    }
    bail!("No session for user {user_name} found")
}

fn check_if_user_has_session(user_name: &str, query_user_stdout: &str) -> bool {
    for line in query_user_stdout.lines().skip(1) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(user_name_of_session) = words.first() else {
            continue;
        };
        if &user_name == user_name_of_session
            || &format!(
                // `>` marks the current session
                ">{user_name}"
            ) == user_name_of_session
        {
            return true;
        }
    }
    false
}

#[cfg_attr(test, derive(Debug, PartialEq))]
struct Paths {
    script: Utf8PathBuf,
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
            stdout: Utf8PathBuf::from(format!("{base_path}.stdout")),
            stderr: Utf8PathBuf::from(format!("{base_path}.stderr")),
            exit_code: Utf8PathBuf::from(format!("{base_path}.exit_code")),
        }
    }
}

struct TaskManager {
    task_service: TaskScheduler::ITaskService,
    task_folder: TaskScheduler::ITaskFolder,
    _co_uninit_guard: winsafe::guard::CoUninitializeGuard,
}

impl TaskManager {
    // Ideally, we would use winsafe only, but Settings and SetDisallowStartIfOnBatteries is
    // apparently not supporte by winsafe
    fn new() -> AnyhowResult<Self> {
        let _co_uninit_guard = winsafe::CoInitializeEx(
            winsafe::co::COINIT::MULTITHREADED | winsafe::co::COINIT::DISABLE_OLE1DDE,
        )?;
        unsafe {
            let task_service: TaskScheduler::ITaskService =
                Com::CoCreateInstance(&TaskScheduler::TaskScheduler, None, Com::CLSCTX_ALL)?;
            task_service.Connect(
                VARIANT::default(),
                VARIANT::default(),
                VARIANT::default(),
                VARIANT::default(),
            )?;
            let task_folder = task_service.GetFolder(&BSTR::from("\\"))?;
            Ok(Self {
                task_service,
                task_folder,
                _co_uninit_guard,
            })
        }
    }

    fn create_task(
        &self,
        name: &str,
        path_executable: &Utf8Path,
        user_id: &str,
    ) -> WinApiResult<TaskScheduler::IRegisteredTask> {
        debug!("Creating task {name}");
        unsafe {
            let task_definition = self.task_service.NewTask(0)?;

            let exec_action: TaskScheduler::IExecAction = task_definition
                .Actions()?
                .Create(TaskScheduler::TASK_ACTION_EXEC)?
                .cast()?;
            exec_action.SetPath(&BSTR::from(path_executable.as_str()))?;

            let principal = task_definition.Principal()?;
            principal.SetUserId(&BSTR::from(user_id))?;

            let settings = task_definition.Settings()?;
            settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;

            self.task_folder.RegisterTaskDefinition(
                &BSTR::from(name),
                &task_definition,
                TaskScheduler::TASK_CREATE_OR_UPDATE.0,
                VARIANT::default(),
                VARIANT::default(),
                TaskScheduler::TASK_LOGON_INTERACTIVE_TOKEN,
                VARIANT::default(),
            )
        }
    }

    #[tokio::main]
    async fn run_task(
        &self,
        task: &TaskScheduler::IRegisteredTask,
        timeout: u64,
        cancellation_token: &CancellationToken,
    ) -> WinApiResult<Outcome<WinApiResult<()>>> {
        let (name, running_task) = unsafe {
            let name = task.Name()?;
            debug!("Starting task {}", name);
            (name, task.Run(VARIANT::default())?)
        };
        let outcome = waited(
            Duration::from_secs(timeout),
            cancellation_token,
            self.await_task_completion(task),
        )
        .await;
        if !matches!(outcome, Outcome::Completed(Ok(_))) {
            error!("Killing task {}", name);
            let _ = self.kill_task(&running_task);
        }
        Ok(outcome)
    }

    fn delete_task(&self, task: &TaskScheduler::IRegisteredTask) -> WinApiResult<()> {
        unsafe {
            let name = task.Name()?;
            debug!("Deleting task {name}");
            self.task_folder.DeleteTask(&name, 0)
        }
    }

    // Note: We explicitly don't use IRunningTask to check if the task is still running. This object
    // produces an error once the task is not running anymore:
    // Error { code: HRESULT(0x8004130B), message: "There is no running instance of the task." }
    // This means that we cannot distinguish between an actual error and an error because the task
    // completed.
    async fn await_task_completion(
        &self,
        task: &TaskScheduler::IRegisteredTask,
    ) -> WinApiResult<()> {
        unsafe {
            debug!("Waiting for task {} to complete", task.Name()?);
            while task.GetInstances(0)?.Count()? > 0 {
                yield_now().await
            }
        }
        Ok(())
    }

    fn kill_task(&self, running_task: &TaskScheduler::IRunningTask) -> WinApiResult<()> {
        let pid = unsafe {
            running_task.Refresh()?;
            running_task.EnginePID()?
        };
        kill_process_tree(&sysinfo::Pid::from_u32(pid));
        Ok(())
    }
}

fn build_task_script(task_name: &str, command_spec: &CommandSpec, paths: &Paths) -> String {
    [
        String::from("@echo off"),
        format!("echo Robotmk: running task {task_name}. Please do not close this window."),
        format!("{command_spec} > {} 2> {}", paths.stdout, paths.stderr),
        format!("echo %errorlevel% > {}", paths.exit_code),
    ]
    .join("\n")
}

fn read_exit_code(path: &Utf8Path) -> AnyhowResult<i32> {
    let content = fs::read_to_string(path).context(format!(
        "Failed to read task exit code file {path}. Probable causes: task was killed or session is \
        inactive."
    ))?;
    let content_until_first_whitespace = content
        .split_whitespace()
        .next()
        .context(format!("{path} is empty"))?
        .to_string();
    content_until_first_whitespace.parse().context(format!(
        "Failed to parse {} as i32",
        content_until_first_whitespace
    ))
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
                exit_code: Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0.exit_code"),
            }
        )
    }

    #[test]
    fn check_if_user_has_session_ok() {
        assert!(check_if_user_has_session(
            "vagrant",
            " USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
>vagrant               console             1  Active      none   12/4/2023 9:35 AM
 vagrant2              rdp-tcp#0           2  Active          .  12/4/2023 9:36 AM"
        ))
    }

    #[test]
    fn check_if_user_has_session_disconnected() {
        assert!(check_if_user_has_session(
            "vagrant2",
            " USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
>vagrant               console             1  Active      none   12/4/2023 9:35 AM
 vagrant2                                  2  Disc            .  12/4/2023 9:36 AM"
        ))
    }

    #[test]
    fn check_if_user_has_session_no_session() {
        assert!(!check_if_user_has_session(
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
                "robotmk_task",
                &command_spec,
                &Paths::from(Utf8PathBuf::from("C:\\working\\suites\\my_suite\\123\\0").as_ref())
            ),
            "@echo off
echo Robotmk: running task robotmk_task. Please do not close this window.
\"C:\\\\somewhere\\\\rcc.exe\" \"mandatory\" \"--some-flag\" \"--some-option\" \"some-value\" \
> C:\\working\\suites\\my_suite\\123\\0.stdout 2> C:\\working\\suites\\my_suite\\123\\0.stderr
echo %errorlevel% > C:\\working\\suites\\my_suite\\123\\0.exit_code"
        )
    }

    #[test]
    fn read_exit_code_ok() -> AnyhowResult<()> {
        let temp_path = NamedTempFile::new()?.into_temp_path();
        write(&temp_path, "123\n456")?;
        assert_eq!(
            read_exit_code(&Utf8PathBuf::try_from(temp_path.to_path_buf())?)?,
            123
        );
        Ok(())
    }

    #[test]
    fn read_exit_code_empty() -> AnyhowResult<()> {
        assert!(format!(
            "{:?}",
            read_exit_code(&Utf8PathBuf::try_from(
                NamedTempFile::new()?.into_temp_path().to_path_buf(),
            )?)
            .err()
            .unwrap()
        )
        .contains("is empty"));
        Ok(())
    }
}
