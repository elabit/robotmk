use camino::Utf8PathBuf;
use clap::{ArgAction, Parser};
use flexi_logger::LogSpecification;

#[derive(Parser)]
#[command(about = "Robotmk suite scheduler.")]
pub struct Args {
    /// Configuration file path.
    #[arg(name = "CONFIG_PATH")]
    pub config_path: Utf8PathBuf,

    /// Log file path. If left unspecified, the program will log to standard error.
    #[arg(long, name = "LOG_PATH")]
    pub log_path: Option<Utf8PathBuf>,

    /// Run flag file. If specified, the program will terminate as soon as this file does not exist.
    #[arg(long, name = "RUN_FLAG")]
    pub run_flag: Option<Utf8PathBuf>,

    /// Enable verbose output. Use once (-v) for logging level INFO and twice (-vv) for logging
    /// level DEBUG.
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,
}

impl Args {
    pub fn log_specification(&self) -> LogSpecification {
        match self.verbose {
            2.. => LogSpecification::debug(),
            1 => LogSpecification::info(),
            _ => LogSpecification::warn(),
        }
    }
}
