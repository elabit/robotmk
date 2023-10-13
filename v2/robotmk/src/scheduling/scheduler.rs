use super::suites::{run_suite, try_acquire_suite_lock};
use crate::config::internal::{GlobalConfig, Suite};
use crate::logging::log_and_return_error;

use anyhow::{bail, Result};
use clokwerk::{Scheduler, TimeUnits};
use log::error;
use std::thread::{sleep, spawn};
use std::time::Duration;

pub fn run_suites(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    let mut scheduler = Scheduler::new();
    let suites_owned: Vec<Suite> = suites.to_vec();

    for suite in suites_owned {
        scheduler
            .every(suite.execution_config.execution_interval_seconds.seconds())
            .run(move || run_suite_in_new_thread(suite.clone()));
    }

    loop {
        if global_config.termination_flag.should_terminate() {
            error!("Received termination signal while scheduling, waiting for suites to terminate");
            wait_until_all_suites_have_terminated(suites);
            bail!("Terminated");
        }
        scheduler.run_pending();
        sleep(Duration::from_millis(250));
    }
}

fn run_suite_in_new_thread(suite: Suite) {
    spawn(move || run_suite(&suite).map_err(log_and_return_error));
}

fn wait_until_all_suites_have_terminated(suites: &[Suite]) {
    let mut still_running_suites: Vec<&Suite> = suites.iter().collect();
    while !still_running_suites.is_empty() {
        let mut still_running = vec![];
        for suite in still_running_suites {
            let _ = try_acquire_suite_lock(suite).map_err(|_| {
                error!("Suite {} is still running", suite.name);
                still_running.push(suite)
            });
        }
        still_running_suites = still_running;
        sleep(Duration::from_millis(250));
    }
}