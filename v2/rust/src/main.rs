#![allow(dead_code)]
pub mod attempt;
mod cli;
mod config;
mod environment;
mod logging;
pub mod parse_xml;
mod results;
mod setup;
mod termination;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info, warn};
use logging::log_and_return_error;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<()> {
    run().map_err(log_and_return_error)?;
    Ok(())
}

fn run() -> Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), &args.log_path)?;
    info!("Program started and logging set up");

    let conf = config::load(&args.config_path).context("Configuration loading failed")?;
    debug!("Configuration loaded");

    setup::setup(&conf).context("Setup failed")?;
    debug!("Setup completed");

    let termination_flag =
        termination::start_termination_control().context("Failed to set up termination control")?;
    debug!("Termination control set up");

    loop {
        if termination_flag.should_terminate() {
            warn!("Termination signal received, shutting down");
            exit(1);
        }
        sleep(Duration::from_millis(100))
    }
}
