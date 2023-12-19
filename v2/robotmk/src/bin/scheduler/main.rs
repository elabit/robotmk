mod build;
mod cli;
mod internal_config;
mod logging;
mod scheduling;
mod setup;
mod termination;

use anyhow::{bail, Context, Result};
use clap::Parser;
use log::info;
use logging::log_and_return_error;
use robotmk::lock::Locker;
use robotmk::results::SchedulerPhase;
use robotmk::section::WriteSection;

fn main() -> Result<()> {
    run().map_err(log_and_return_error)?;
    Ok(())
}

fn run() -> Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), args.log_path)?;
    info!("Program started and logging set up");

    let external_config =
        robotmk::config::load(&args.config_path).context("Configuration loading failed")?;
    info!("Configuration loaded");

    let cancellation_token = termination::start_termination_control(args.run_flag)
        .context("Failed to set up termination control")?;
    info!("Termination control set up");

    let (global_config, suites) = internal_config::from_external_config(
        external_config,
        cancellation_token.clone(),
        Locker::new(&args.config_path, Some(&cancellation_token)),
    );

    if global_config.cancellation_token.is_cancelled() {
        bail!("Terminated")
    }

    setup::general::setup(&global_config, &suites).context("General setup failed")?;
    info!("General setup completed");
    write_phase(&SchedulerPhase::RCCSetup, &global_config)?;
    let suites = setup::rcc::setup(&global_config, suites).context("RCC-specific setup failed")?;
    info!("RCC-specific setup completed");

    if global_config.cancellation_token.is_cancelled() {
        bail!("Terminated")
    }

    info!("Starting environment building");
    write_phase(&SchedulerPhase::EnvironmentBuilding, &global_config)?;
    let suites = build::build_environments(&global_config, suites)?;
    info!("Environment building finished");

    if global_config.cancellation_token.is_cancelled() {
        bail!("Terminated")
    }

    info!("Starting suite scheduling");
    write_phase(&SchedulerPhase::Scheduling, &global_config)?;
    scheduling::scheduler::run_suites_and_cleanup(&global_config, &suites)
}

fn write_phase(
    phase: &SchedulerPhase,
    global_config: &internal_config::GlobalConfig,
) -> Result<()> {
    phase.write(
        global_config.results_directory.join("scheduler_phase.json"),
        &global_config.results_directory_locker,
    )
}
