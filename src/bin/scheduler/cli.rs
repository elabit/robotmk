use camino::Utf8PathBuf;
use clap::{ArgAction, Parser};
use flexi_logger::LogSpecification;

#[derive(Parser)]
#[command(about = "Robotmk scheduler.", version)]
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

    /// Grace period. If specified, the program will sleep for this amount of seconds after
    /// completing some general setup steps to give the system some time to prepare (eg. session
    /// creation).
    #[arg(long, name = "GRACE_PERIOD")]
    pub grace_period: Option<u64>,

    /// Plan id. If specified, only this plan will be executed.
    /// If not specified, all plans will be executed.
    #[arg(long, name = "PLAN")]
    pub plan: Option<String>,

    /// No JSON plan report will be produced.
    #[arg(long = "no-plan-result")]
    pub no_plan_result: bool,
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
