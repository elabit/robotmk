use super::cleanup::cleanup_working_directories;
use super::suites::{run_suite, try_acquire_suite_lock};
use crate::internal_config::{GlobalConfig, Suite};
use crate::logging::log_and_return_error;

use anyhow::{bail, Result};
use log::error;
use std::thread::sleep;
use std::time::Duration;
use tokio::task::spawn_blocking;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

#[tokio::main]
pub async fn run_suites_and_cleanup(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    let suites_for_scheduling: Vec<Suite> = suites.to_vec();

    for suite in suites_for_scheduling {
        tokio::spawn(run_suite_scheduler(suite));
    }

    tokio::spawn(run_cleanup_job(
        global_config.cancellation_token.clone(),
        suites.to_vec(),
    ));

    global_config.cancellation_token.cancelled().await;
    error!("Received termination signal while scheduling, waiting for suites to terminate");
    wait_until_all_suites_have_terminated(suites);
    bail!("Terminated");
}

async fn run_suite_scheduler(suite: Suite) {
    let mut clock = interval(Duration::from_secs(suite.execution_interval_seconds));
    loop {
        let suite = suite.clone();
        tokio::select! {
            _ = clock.tick() => { }
            _ = suite.cancellation_token.cancelled() => { return }
        };
        spawn_blocking(move || run_suite(&suite).map_err(log_and_return_error));
    }
}

async fn run_cleanup_job(cancellation_token: CancellationToken, suites: Vec<Suite>) {
    let mut clock = interval(Duration::from_secs(300));
    loop {
        let suites = suites.clone();
        tokio::select! {
            _ = clock.tick() => { }
            _ = cancellation_token.cancelled() => { return }
        };
        spawn_blocking(move || cleanup_working_directories(suites.iter()));
    }
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
