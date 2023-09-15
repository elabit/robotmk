use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Robotmk suite scheduler.")]
pub struct Args {
    /// Configuration file path.
    #[arg(name = "CONFIG_PATH")]
    pub config_path: PathBuf,

    /// Log file path. If left unspecified, the program will log to standard error.
    #[arg(long, name = "LOG_PATH")]
    pub log_path: Option<PathBuf>,

    /// Enable verbose output. Use once (-v) for logging level INFO and twice (-vv) for logging
    /// level DEBUG.
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}
