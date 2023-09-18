#![allow(dead_code)]
pub mod attempt;
mod cli;
mod logging;

use clap::Parser;
use log::info;
pub mod parse_xml;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), &args.log_path)?;
    info!("Program started and logging set up");

    Ok(()).map_err(logging::log_and_return_error)
}
