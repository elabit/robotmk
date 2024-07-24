use super::lock::{Locker, LockerError};

use anyhow::{Context, Result as AnyhowResult};
use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Host {
    Piggyback(String),
    Source,
}

#[derive(Deserialize, Serialize)]
pub struct Section {
    pub host: Host,
    pub name: String,
    pub content: String,
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Failed to `{0}`")]
    Unrecoverable(String),
    #[error("Terminated")]
    Cancelled,
}

impl From<LockerError> for WriteError {
    fn from(value: LockerError) -> Self {
        match value {
            LockerError::Cancelled => Self::Cancelled,
            value => Self::Unrecoverable(format!("{:?}", value)),
        }
    }
}

impl From<anyhow::Error> for WriteError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unrecoverable(format!("{:?}", value))
    }
}

fn write(section: &Section, path: impl AsRef<Utf8Path>, locker: &Locker) -> Result<(), WriteError> {
    let path = path.as_ref();
    let section = serde_json::to_string(&section).unwrap();
    let mut file = NamedTempFile::new().context("Opening tempfile failed")?;
    file.write_all(section.as_bytes()).context(format!(
        "Writing tempfile failed, {}",
        file.path().display()
    ))?;

    let lock = locker.wait_for_write_lock()?;
    file.persist(path)
        .context(format!("Persisting tempfile failed, final_path: {path}"))?;
    Ok(lock.release()?)
}

pub trait WriteSection {
    fn name() -> &'static str;

    fn write(&self, path: impl AsRef<Utf8Path>, locker: &Locker) -> Result<(), WriteError>
    where
        Self: Serialize,
    {
        let section = Section {
            name: Self::name().into(),
            content: serde_json::to_string(&self).unwrap(),
            host: Host::Source,
        };
        write(&section, path, locker)
    }
}

pub trait WritePiggybackSection {
    fn name() -> &'static str;

    fn write(
        &self,
        path: impl AsRef<Utf8Path>,
        host: Host,
        locker: &Locker,
    ) -> Result<(), WriteError>
    where
        Self: Serialize,
    {
        let section = Section {
            name: Self::name().into(),
            content: serde_json::to_string(&self).unwrap(),
            host,
        };
        write(&section, path, locker)
    }
}

fn read_entry(entry: Result<DirEntry, walkdir::Error>) -> AnyhowResult<Section> {
    let entry = entry?;
    let raw = read_to_string(entry.path())?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn read(directory: impl AsRef<Path>, locker: &Locker) -> Result<Vec<Section>, LockerError> {
    // TODO: Test this function.
    let lock = locker.wait_for_read_lock()?;
    let sections = WalkDir::new(directory)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| read_entry(entry).ok())
        .collect();
    lock.release()?;
    Ok(sections)
}
