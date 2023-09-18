#![allow(dead_code)]
pub mod attempt;
mod cli;
mod config;
mod environment;
mod logging;
pub mod parse_xml;
mod results;
mod setup;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
use logging::log_and_return_error;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), &args.log_path)?;
    info!("Program started and logging set up");

    let conf = config::load(&args.config_path)
        .context("Configuration loading failed")
        .map_err(log_and_return_error)?;
    debug!("Configuration loaded");

    setup::setup(&conf)
        .context("Setup failed")
        .map_err(log_and_return_error)?;
    debug!("Setup completed");

    Ok(())
}
