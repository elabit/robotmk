use camino::Utf8PathBuf;
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, FileSpec, FlexiLoggerError, LogSpecification, Logger,
    LoggerHandle, Naming, Record,
};
use log::error;
use std::fmt::Debug;

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
    .rotate(
        Criterion::Age(Age::Day),
        Naming::Numbers,
        Cleanup::KeepLogFiles(14),
    )
    .start()
}

pub fn log_and_return_error<T>(error: T) -> T
where
    T: Debug,
{
    error!("{error:?}");
    error
}
