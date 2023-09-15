#![allow(dead_code)]
pub mod attempt;
mod cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let _args = cli::Args::parse();
    Ok(())
}
