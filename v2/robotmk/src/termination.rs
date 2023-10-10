use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sysinfo::{Pid, Process, ProcessExt, System, SystemExt};

pub fn start_termination_control() -> Result<TerminationFlag> {
    let raw_flag = Arc::new(AtomicBool::new(false));
    let raw_flag_clone = raw_flag.clone();
    ctrlc::set_handler(move || {
        raw_flag_clone.store(true, Ordering::Relaxed);
    })
    .context("Failed to register signal handler for CTRL+C")?;
    Ok(TerminationFlag(raw_flag))
}

#[derive(Clone)]
pub struct TerminationFlag(Arc<AtomicBool>);

impl TerminationFlag {
    pub fn should_terminate(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
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

    impl TerminationFlag {
        pub fn new() -> Self {
            Self(Arc::new(AtomicBool::new(false)))
        }
    }
}
