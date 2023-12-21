use anyhow::{bail, Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use fs4::FileExt;
use log::debug;
use std::fs::File;
use std::io;
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Locker {
    lock_path: Utf8PathBuf,
    cancellation_token: CancellationToken,
}

pub struct Lock(File);

impl Locker {
    pub fn new(
        lock_path: impl AsRef<Utf8Path>,
        cancellation_token: Option<&CancellationToken>,
    ) -> Self {
        Self {
            lock_path: lock_path.as_ref().to_owned(),
            cancellation_token: cancellation_token.cloned().unwrap_or_default(),
        }
    }

    pub fn wait_for_read_lock(&self) -> AnyhowResult<Lock> {
        debug!("Waiting for read lock");
        let file = self.file()?;
        let file = with_cancellation(
            || file.lock_shared().map(|_| file),
            &self.cancellation_token,
        )
        .context("Failed to acquire read lock")?;
        debug!("Got read lock");
        Ok(Lock(file))
    }

    pub fn wait_for_write_lock(&self) -> AnyhowResult<Lock> {
        debug!("Waiting for write lock");
        let file = self.file()?;
        let file = with_cancellation(
            || file.lock_exclusive().map(|_| file),
            &self.cancellation_token,
        )
        .context("Failed to acquire write lock")?;
        debug!("Got write lock");
        Ok(Lock(file))
    }

    fn file(&self) -> AnyhowResult<File> {
        File::open(&self.lock_path).context(format!(
            "Failed to open {} for creating lock",
            self.lock_path,
        ))
    }
}

#[tokio::main]
async fn with_cancellation<F>(lock: F, cancellation_token: &CancellationToken) -> AnyhowResult<File>
where
    F: FnOnce() -> io::Result<File> + Send + 'static,
{
    tokio::select! {
        file = spawn_blocking(lock) => { Ok(file??) }
        _ = cancellation_token.cancelled() => { bail!("Terminated") }
    }
}

impl Lock {
    pub fn release(self) -> AnyhowResult<()> {
        self.0.unlock().context("Failed to release lock")
    }
}
