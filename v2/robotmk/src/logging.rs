use anyhow::Error;
use camino::Utf8PathBuf;
use flexi_logger::{
    detailed_format, FileSpec, FlexiLoggerError, LogSpecification, Logger, LoggerHandle,
};
use log::error;

pub fn init(
    specification: LogSpecification,
    path: &Option<Utf8PathBuf>,
) -> Result<LoggerHandle, FlexiLoggerError> {
    let logger = Logger::with(specification);
    match path {
        Some(path) => logger.log_to_file(FileSpec::try_from(path)?),
        None => logger.log_to_stderr(),
    }
    .format(detailed_format)
    .start()
}

pub fn log_and_return_error(error: Error) -> Error {
    error!("{error:?}");
    error
}
