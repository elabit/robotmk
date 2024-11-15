use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, Process, ProcessesToUpdate, System};

fn get_children(processes: &HashMap<Pid, Process>, pid: &Pid) -> HashSet<Pid> {
    processes
        .iter()
        .filter_map(|(child_pid, process)| {
            ((process.parent().as_ref() == Some(pid)) && process.thread_kind().is_none())
                .then_some(*child_pid)
        })
        .collect()
}

fn print_process_tree(depth: usize, processes: &HashMap<Pid, Process>, pid: Pid, max_depth: usize) {
    let process = processes.get(&pid).unwrap();
    println!(
        "{}{pid} {:?} {:?} {:?}",
        "-".repeat(depth),
        process.status(),
        process.name(),
        process.exe(),
    );
    if depth >= max_depth {
        return;
    }
    for child in get_children(processes, &pid) {
        print_process_tree(depth + 1, processes, child, max_depth);
    }
}

fn get_tree_size(processes: &HashMap<Pid, Process>, pid: Pid) -> usize {
    #[cfg(windows)]
    let mut count = {
        let status = processes.get(&pid).unwrap().status();
        use sysinfo::ProcessStatus;
        match status {
            ProcessStatus::Zombie => 0,
            _ => 1,
        }
    };
    #[cfg(unix)]
    let mut count = 1;
    for child in get_children(processes, &pid) {
        count += get_tree_size(processes, child);
    }
    count
}

pub fn check_tree_size(system: &mut System, pid: Pid) -> usize {
    system.refresh_processes(ProcessesToUpdate::All, true);
    let processes = system.processes();
    println!("Process tree");
    print_process_tree(0, processes, pid, 5);
    get_tree_size(processes, pid)
}
