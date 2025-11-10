mod build;
mod cli;
mod internal_config;
mod logging;
mod scheduling;
mod setup;
mod termination;

use anyhow::{Context, Result as AnyhowResult};
use clap::Parser;
use log::info;
use logging::log_and_return_error;
use robotmk::lock::Locker;
use robotmk::results::{SchedulerPhase, SetupFailure, SetupFailures};
use robotmk::section::WriteSection;
use robotmk::termination::Terminate;
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use tokio_util::sync::CancellationToken;

fn main() -> AnyhowResult<()> {
    if let Err(e) = run() {
        return match e {
            Terminate::Cancelled => {
                info!("Terminated");
                Ok(())
            }
            Terminate::Unrecoverable(any) => Err(log_and_return_error(any)),
        };
    }
    Ok(())
}

fn run() -> Result<(), Terminate> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), args.log_path).context("Logging setup failed.")?;
    info!("Program started and logging set up");

    let external_config =
        robotmk::config::load(&args.config_path).context("Configuration loading failed")?;
    info!("Configuration loaded");

    if let Some(plan_id) = &args.plan {
        info!("Filtering configuration to only include plan: {}", plan_id);
    }
    let filtered_external_config =
        robotmk::config::filter_by_plan_id(external_config, args.plan.as_deref());

    let cancellation_token = termination::start_termination_control(args.run_flag)
        .context("Failed to set up termination control")?;
    info!("Termination control set up");

    let (global_config, plans) = internal_config::from_external_config(
        filtered_external_config,
        &cancellation_token,
        &Locker::new(&args.config_path, Some(&cancellation_token)),
    );

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    if let Some(grace_period) = args.grace_period {
        info!("Grace period: Sleeping for {grace_period} seconds");
        write_phase(&SchedulerPhase::GracePeriod(grace_period), &global_config)?;
        await_grace_period(grace_period, &cancellation_token);
    }

    setup::base_directories::setup(&global_config, &plans)?;
    info!("Base setup completed");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    write_phase(&SchedulerPhase::Setup, &global_config)?;
    let (plans, setup_failures) = setup::steps::run::run(&global_config, plans)?;
    write_setup_failures(setup_failures.into_iter(), &global_config)?;
    info!("Setup steps completed");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    info!("Starting environment building");
    write_phase(&SchedulerPhase::EnvironmentBuilding, &global_config)?;
    let plans = build::build_environments(&global_config, plans)?;
    info!("Environment building finished");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    info!("Starting plan scheduling");
    write_phase(&SchedulerPhase::Scheduling, &global_config)?;
    scheduling::scheduler::run_plans_and_cleanup(&global_config, &plans);

    Err(Terminate::Cancelled)
}

fn write_phase(
    phase: &SchedulerPhase,
    global_config: &internal_config::GlobalConfig,
) -> Result<(), Terminate> {
    phase.write(
        global_config.results_directory.join("scheduler_phase.json"),
        &global_config.results_directory_locker,
    )
}

fn write_setup_failures(
    failures: impl Iterator<Item = SetupFailure>,
    global_config: &internal_config::GlobalConfig,
) -> Result<(), Terminate> {
    SetupFailures(failures.collect()).write(
        global_config.results_directory.join("setup_failures.json"),
        &global_config.results_directory_locker,
    )
}

#[tokio::main]
async fn await_grace_period(grace_period: u64, cancellation_token: &CancellationToken) {
    let _ = timeout_at(
        Instant::now() + Duration::from_secs(grace_period),
        cancellation_token.cancelled(),
    )
    .await;
}
