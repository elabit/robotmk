mod child_process_supervisor;
mod cli;
mod command_spec;
mod environment;
mod internal_config;
mod logging;
mod results;
mod rf;
mod scheduling;
mod sessions;
mod setup;
mod termination;

use anyhow::{bail, Context, Result};
use clap::Parser;
use log::{debug, info};
use logging::log_and_return_error;
use robotmk::lock::Locker;
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
    debug!("Configuration loaded");

    let termination_flag = termination::start_termination_control(args.run_flag)
        .context("Failed to set up termination control")?;
    debug!("Termination control set up");

    let (global_config, suites) = internal_config::from_external_config(
        external_config,
        termination_flag.clone(),
        Locker::new(&args.config_path, Some(&termination_flag)),
    );

    if global_config.termination_flag.should_terminate() {
        bail!("Terminated")
    }

    setup::general::setup(&global_config, &suites).context("General setup failed")?;
    debug!("General setup completed");
    write_phase(&results::SchedulerPhase::RCCSetup, &global_config)?;
    let suites = setup::rcc::setup(&global_config, suites).context("RCC-specific setup failed")?;
    debug!("RCC-specific setup completed");

    if global_config.termination_flag.should_terminate() {
        bail!("Terminated")
    }

    info!("Starting environment building");
    write_phase(
        &results::SchedulerPhase::EnvironmentBuilding,
        &global_config,
    )?;
    let suites = environment::build_environments(&global_config, suites)?;
    info!("Environment building finished");

    if global_config.termination_flag.should_terminate() {
        bail!("Terminated")
    }

    info!("Starting suite scheduling");
    write_phase(&results::SchedulerPhase::Scheduling, &global_config)?;
    scheduling::scheduler::run_suites_and_cleanup(&global_config, &suites)
}

fn write_phase(
    phase: &results::SchedulerPhase,
    global_config: &internal_config::GlobalConfig,
) -> Result<()> {
    phase.write(
        global_config.results_directory.join("scheduler_phase.json"),
        &global_config.results_directory_locker,
    )
}
