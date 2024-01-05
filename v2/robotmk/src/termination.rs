use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::time::Duration;
use sysinfo::{Pid, Process, System};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cancelled")
    }
}

impl Error for Cancelled {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub enum Outcome<T> {
    Cancel,
    Timeout,
    Completed(T),
}

pub async fn waited<F>(
    duration: Duration,
    flag: &CancellationToken,
    future: F,
) -> Outcome<F::Output>
where
    F: Future,
{
    tokio::select! {
        outcome = future => { Outcome::Completed(outcome) },
        _ = flag.cancelled() => { Outcome::Cancel },
        _ = sleep(duration) => { Outcome::Timeout },
    }
}

// This is a non-cooperative termination (SIGKILL) of the entire process tree. What we would
// actually like to do is to shut down our child co-operatively and leave the termination of any
// non-direct children further down the tree to our child. However, Windows offers no API for this
// (there is no SIGTERM on Windows), so we instead kill the entire tree.
pub fn kill_process_tree(top_pid: &Pid) {
    let mut system = System::new();
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
