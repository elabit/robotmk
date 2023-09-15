#![allow(dead_code)]
pub mod attempt;
mod cli;
mod config;
mod logging;
pub mod parse_xml;

use anyhow::Context;
use clap::Parser;
use log::{debug, info};
use logging::log_and_return_error;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), &args.log_path)?;
    info!("Program started and logging set up");

    let _config = config::load(&args.config_path)
        .context("Configuration loading failed")
        .map_err(log_and_return_error)?;
    debug!("Configuration loaded");

    Ok(())
}
