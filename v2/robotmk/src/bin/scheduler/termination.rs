use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use log::debug;
use std::thread::{sleep, spawn};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub fn start_termination_control(run_flag_file: Option<Utf8PathBuf>) -> Result<CancellationToken> {
    let token = CancellationToken::new();
    watch_ctrlc(token.clone()).context("Failed to register signal handler for CTRL+C")?;
    if let Some(run_flag_file) = run_flag_file {
        start_run_flag_watch_thread(run_flag_file, token.clone());
    }
    Ok(token)
}

fn watch_ctrlc(token: CancellationToken) -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(move || token.cancel())
}

fn start_run_flag_watch_thread(file: Utf8PathBuf, token: CancellationToken) {
    spawn(move || {
        debug!("Watching {file}");
        while file.exists() {
            sleep(Duration::from_millis(250));
        }
        debug!("{file} not found, raising termination flag");
        token.cancel()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn run_flag_file() -> Result<()> {
        let run_flag_temp_path = NamedTempFile::new()?.into_temp_path();
        let cancellation_token = start_termination_control(Some(Utf8PathBuf::try_from(
            run_flag_temp_path.to_path_buf(),
        )?))?;
        run_flag_temp_path.close()?;
        sleep(Duration::from_millis(500));
        assert!(cancellation_token.is_cancelled());
        Ok(())
    }
}
