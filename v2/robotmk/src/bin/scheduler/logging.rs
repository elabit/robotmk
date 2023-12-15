use anyhow::Error;
use camino::Utf8PathBuf;
use flexi_logger::{
    DeferredNow, FileSpec, FlexiLoggerError, LogSpecification, Logger, LoggerHandle, Record,
};
use log::error;

pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H.%M.%S%.f%z";

pub fn format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] {} [{}] {}:{}: {}",
        now.now_utc_owned().format(TIMESTAMP_FORMAT),
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

pub fn init(
    specification: LogSpecification,
    path: Option<Utf8PathBuf>,
) -> Result<LoggerHandle, FlexiLoggerError> {
    let logger = Logger::with(specification);
    match path {
        Some(path) => logger.log_to_file(FileSpec::try_from(path)?),
        None => logger.log_to_stderr(),
    }
    .format(format)
    .start()
}

pub fn log_and_return_error(error: Error) -> Error {
    error!("{error:?}");
    error
}
