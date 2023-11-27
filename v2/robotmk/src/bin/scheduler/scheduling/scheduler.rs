use super::cleanup::cleanup_working_directories;
use super::suites::{run_suite, try_acquire_suite_lock};
use crate::internal_config::{GlobalConfig, Suite};
use crate::logging::log_and_return_error;

use anyhow::{bail, Result};
use clokwerk::{Scheduler, TimeUnits};
use log::error;
use std::thread::{sleep, spawn};
use std::time::Duration;

pub fn run_suites_and_cleanup(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    let mut scheduler = Scheduler::new();
    let suites_for_scheduling: Vec<Suite> = suites.to_vec();

    for suite in suites_for_scheduling {
        scheduler
            .every(suite.execution_interval_seconds.seconds())
            .run(move || run_suite_in_new_thread(suite.clone()));
    }

    let suites_for_cleanup: Vec<Suite> = suites.to_vec();
    scheduler
        .every(5.minutes())
        .run(move || run_cleanup_working_directories_in_new_thread(suites_for_cleanup.clone()));

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

fn run_cleanup_working_directories_in_new_thread(suites: Vec<Suite>) {
    spawn(move || cleanup_working_directories(suites.iter()));
}

fn wait_until_all_suites_have_terminated(suites: &[Suite]) {
    let mut still_running_suites: Vec<&Suite> = suites.iter().collect();
    while !still_running_suites.is_empty() {
        let mut still_running = vec![];
        for suite in still_running_suites {
            let _ = try_acquire_suite_lock(suite).map_err(|_| {
                error!("Suite {} is still running", suite.id);
                still_running.push(suite)
            });
        }
        still_running_suites = still_running;
        sleep(Duration::from_millis(250));
    }
}
