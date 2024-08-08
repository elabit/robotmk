use super::cleanup::cleanup_working_directories;
use super::plans::run_plan;
use crate::internal_config::{GlobalConfig, Plan};
use crate::logging::log_and_return_error;

use chrono::Utc;
use log::error;
use std::collections::HashMap;
use std::time::Duration;
use tokio::task::{spawn_blocking, JoinSet};
use tokio::time::{interval_at, Instant};
use tokio_util::sync::CancellationToken;

#[tokio::main]
pub async fn run_plans_and_cleanup(global_config: &GlobalConfig, plans: &[Plan]) {
    let mut plans_by_exec_group = HashMap::new();
    for plan in plans {
        plans_by_exec_group
            .entry((
                plan.group_affiliation.group_index,
                plan.group_affiliation.execution_interval,
            ))
            .or_insert(vec![])
            .push(plan.clone());
    }

    let mut join_set = JoinSet::new();
    for ((_, execution_interval), mut plans) in plans_by_exec_group {
        plans.sort_by_key(|plan| plan.group_affiliation.position_in_group);
        join_set.spawn(run_sequential_plan_group_scheduler(
            execution_interval,
            plans,
            global_config.cancellation_token.clone(),
        ));
    }

    join_set.spawn(run_cleanup_job(
        global_config.cancellation_token.clone(),
        plans.to_vec(),
    ));

    global_config.cancellation_token.cancelled().await;
    error!("Received termination signal while scheduling, waiting for plans to terminate");
    while let Some(outcome) = join_set.join_next().await {
        if let Err(error) = outcome {
            error!("{error:?}");
        }
    }
}

async fn run_sequential_plan_group_scheduler(
    interval: u64,
    plans: Vec<Plan>,
    cancellation_token: CancellationToken,
) {
    // It is debatable whether MissedTickBehavior::Burst (the default) is correct. In practice, as
    // long as timeout * number of attempts is shorter than the execution interval, it shouldn't
    // make a difference anyway.  However, in case we consider changing this, note that using
    // `MissedTickBehavior::Delay` leads to a strange sort of lag on Windows (as if we added ~10 ms
    // to the scheduling interval). See also:
    // https://www.reddit.com/r/rust/comments/13yymkh/weird_tokiotimeinterval_tick_behavior/
    // https://github.com/tokio-rs/tokio/issues/5021
    let mut clock = interval_at(compute_start_time(interval), Duration::from_secs(interval));
    loop {
        tokio::select! {
            _ = clock.tick() => { }
            _ = cancellation_token.cancelled() => { return }
        };
        for plan in plans.clone() {
            let _ = spawn_blocking(move || run_plan(&plan).map_err(log_and_return_error)).await;
        }
    }
}

async fn run_cleanup_job(cancellation_token: CancellationToken, plans: Vec<Plan>) {
    let mut clock = interval_at(compute_start_time(300), Duration::from_secs(300));
    loop {
        let plans = plans.clone();
        tokio::select! {
            _ = clock.tick() => { }
            _ = cancellation_token.cancelled() => { return }
        };
        spawn_blocking(move || cleanup_working_directories(plans.iter()));
    }
}

fn compute_start_time(execution_interval_secs: u64) -> Instant {
    let now = Instant::now();
    now.checked_add(Duration::from_millis(compute_start_time_offset_millis(
        Utc::now().timestamp_millis() as u64,
        execution_interval_secs * 1000,
    )))
    .unwrap_or(now)
}

fn compute_start_time_offset_millis(now_millis: u64, execution_interval_millis: u64) -> u64 {
    execution_interval_millis - now_millis % execution_interval_millis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_start_time_offset_millis() {
        let now_millis: u64 = 1701942935796;
        let five_min_interval_millis = 5 * 60 * 1000;
        let expected_offset = 264204;
        assert_eq!(
            compute_start_time_offset_millis(now_millis, five_min_interval_millis),
            expected_offset
        );
        assert_eq!((now_millis + expected_offset) % five_min_interval_millis, 0);
        assert!(expected_offset <= five_min_interval_millis);
    }
}
