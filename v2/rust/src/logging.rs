use anyhow::Error;
use flexi_logger::{
    detailed_format, FileSpec, FlexiLoggerError, LogSpecification, Logger, LoggerHandle,
};
use log::error;
use std::path::PathBuf;

pub fn init(
    specification: LogSpecification,
    path: &Option<PathBuf>,
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
    error!(
        "{}",
        error
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    );
    error
}
