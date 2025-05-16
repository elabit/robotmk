use crate::termination::Terminate;
use camino::{Utf8Path, Utf8PathBuf};
use fs4::fs_std::FileExt;
use log::debug;
use std::fs::File;
use std::io;
use thiserror::Error;
use tokio::task::{JoinError, spawn_blocking};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Locker {
    lock_path: Utf8PathBuf,
    cancellation_token: CancellationToken,
}

#[derive(Error, Debug)]
pub enum LockerError {
    #[error("Failed to open `{0}`")]
    Open(Utf8PathBuf, #[source] io::Error),
    #[error("Could not complete task")]
    Join(#[from] JoinError),
    #[error("Terminated")]
    Cancelled,
    #[error("Failed to obtain write lock for `{0}`")]
    Exclusive(Utf8PathBuf, #[source] io::Error),
    #[error("Failed to obtain read lock for `{0}`")]
    Shared(Utf8PathBuf, #[source] io::Error),
    #[error("Failed to release lock for `{0}`")]
    Release(Utf8PathBuf, #[source] io::Error),
}

impl From<LockerError> for Terminate {
    fn from(value: LockerError) -> Self {
        match value {
            LockerError::Cancelled => Self::Cancelled,
            _ => Self::Unrecoverable(value.into()),
        }
    }
}

pub struct Lock(File, Utf8PathBuf);

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

    pub fn wait_for_read_lock(&self) -> Result<Lock, LockerError> {
        debug!("Waiting for read lock");
        let file = self.file()?;
        let lock_path = self.lock_path.clone();
        let file = with_cancellation(
            || {
                FileExt::lock_shared(&file)
                    .map(|_| file)
                    .map_err(|e| LockerError::Shared(lock_path, e))
            },
            &self.cancellation_token,
        )?;
        debug!("Got read lock");
        Ok(Lock(file, self.lock_path.clone()))
    }

    pub fn wait_for_write_lock(&self) -> Result<Lock, LockerError> {
        debug!("Waiting for write lock");
        let file = self.file()?;
        let lock_path = self.lock_path.clone();
        let file = with_cancellation(
            || {
                file.lock_exclusive()
                    .map(|_| file)
                    .map_err(|e| LockerError::Exclusive(lock_path, e))
            },
            &self.cancellation_token,
        )?;
        debug!("Got write lock");
        Ok(Lock(file, self.lock_path.clone()))
    }

    fn file(&self) -> Result<File, LockerError> {
        File::open(&self.lock_path).map_err(|e| LockerError::Open(self.lock_path.clone(), e))
    }
}

#[tokio::main]
async fn with_cancellation<F>(
    lock: F,
    cancellation_token: &CancellationToken,
) -> Result<File, LockerError>
where
    F: FnOnce() -> Result<File, LockerError> + Send + 'static,
{
    tokio::select! {
        file = spawn_blocking(lock) => { file? }
        _ = cancellation_token.cancelled() => { Err(LockerError::Cancelled) }
    }
}

impl Lock {
    pub fn release(self) -> Result<(), LockerError> {
        FileExt::unlock(&self.0).map_err(|e| LockerError::Release(self.1, e))
    }
}
