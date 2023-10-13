#![allow(dead_code)]
pub mod attempt;
mod child_process_supervisor;
mod cli;
mod command_spec;
mod config;
mod environment;
mod logging;
pub mod parse_xml;
mod rebot;
mod results;
mod scheduling;
mod session;
mod setup;
mod termination;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
use logging::log_and_return_error;

fn main() -> Result<()> {
    run().map_err(log_and_return_error)?;
    Ok(())
}

fn run() -> Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), &args.log_path)?;
    info!("Program started and logging set up");

    let conf = config::external::load(&args.config_path).context("Configuration loading failed")?;
    debug!("Configuration loaded");

    setup::setup(&conf).context("Setup failed")?;
    debug!("Setup completed");

    let termination_flag =
        termination::start_termination_control().context("Failed to set up termination control")?;
    debug!("Termination control set up");

    info!("Starting environment building");
    environment::build_environments(&conf, &termination_flag)?;
    info!("Environment building finished");

    info!("Starting suite scheduling");
    scheduling::run_suites(&conf, &termination_flag)
}
