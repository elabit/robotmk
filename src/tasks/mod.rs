use crate::command_spec::CommandSpec;
use crate::termination::Outcome;
use anyhow::Result as AnyhowResult;
use camino::Utf8Path;
use tokio_util::sync::CancellationToken;

#[cfg(windows)]
mod windows;

pub struct TaskSpec<'a> {
    pub task_name: &'a str,
    pub command_spec: &'a CommandSpec,
    pub user_name: &'a str,
    pub runtime_base_path: &'a Utf8Path,
    pub timeout: u64,
    pub cancellation_token: &'a CancellationToken,
}

#[cfg(windows)]
pub fn run_task(task_spec: &TaskSpec) -> AnyhowResult<Outcome<i32>> {
    windows::run_task(task_spec)
}

#[cfg(unix)]
pub fn run_task(_task_spec: &TaskSpec) -> AnyhowResult<Outcome<i32>> {
    panic!("Not implemented")
}
