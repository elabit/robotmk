use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::thread::{sleep, spawn};
use std::time::Duration;
use sysinfo::{Pid, Process, ProcessExt, System, SystemExt};
use tokio_util::sync::CancellationToken;

pub fn start_termination_control(run_flag_file: Option<Utf8PathBuf>) -> Result<CancellationToken> {
    let token = CancellationToken::new();
    watch_ctrlc(token.clone()).context("Failed to register signal handler for CTRL+C")?;
    if let Some(run_flag_file) = run_flag_file {
        start_run_flag_watch_thread(run_flag_file, token.clone());
    }
    Ok(token)
}

fn watch_ctrlc(token: CancellationToken) -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(move || token.cancel())
}

fn start_run_flag_watch_thread(file: Utf8PathBuf, token: CancellationToken) {
    spawn(move || {
        debug!("Watching {file}");
        while file.exists() {
            sleep(Duration::from_millis(250));
        }
        debug!("{file} not found, raising termination flag");
        token.cancel()
    });
}

// This is a non-cooperative termination (SIGKILL) of the entire process tree. What we would
// actually like to do is to shut down our child co-operatively and leave the termination of any
// non-direct children further down the tree to our child. However, Windows offers no API for this
// (there is no SIGTERM on Windows), so we instead kill the entire tree.
pub fn kill_process_tree(top_pid: &Pid) {
    let mut system = System::new_all();
    system.refresh_processes();
    let processes = system.processes();

    match processes.get(top_pid) {
        None => return,
        Some(top_process) => top_process.kill(),
    };

    kill_all_children(top_pid, processes);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn run_flag_file() -> Result<()> {
        let run_flag_temp_path = NamedTempFile::new()?.into_temp_path();
        let cancellation_token = start_termination_control(Some(Utf8PathBuf::try_from(
            run_flag_temp_path.to_path_buf(),
        )?))?;
        run_flag_temp_path.close()?;
        sleep(Duration::from_millis(500));
        assert!(cancellation_token.is_cancelled());
        Ok(())
    }
}
