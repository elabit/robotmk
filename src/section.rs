use super::lock::{Locker, LockerError};

use crate::termination::Terminate;
use anyhow::{Context, Result as AnyhowResult, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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

pub trait WriteSection {
    fn name() -> &'static str;

    fn write(&self, path: impl AsRef<Utf8Path>, locker: &Locker) -> Result<(), Terminate>
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
    ) -> Result<(), Terminate>
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

fn write(section: &Section, path: impl AsRef<Utf8Path>, locker: &Locker) -> Result<(), Terminate> {
    let lock = locker.wait_for_write_lock()?;
    let mut errors = write_to_tmp_and_move(
        &serde_json::to_string(section).unwrap(),
        path.as_ref(),
        &Utf8PathBuf::from(format!("{}.tmp", path.as_ref())),
    );
    if let Err(err) = lock.release() {
        errors.push(anyhow!(err));
    }
    if errors.is_empty() {
        return Ok(());
    }
    let mut error_message =
        "Encountered the following errors while attempting to write section:".to_string();
    for error in errors {
        error_message = format!("{error_message}\n{error:?}");
    }
    Err(Terminate::Unrecoverable(anyhow!(error_message)))
}

fn write_to_tmp_and_move(
    content: &str,
    path: &Utf8Path,
    tmp_path: &Utf8Path,
) -> Vec<anyhow::Error> {
    let mut errors =
        match fs::write(tmp_path, content).context(format!("Writing to {tmp_path} failed")) {
            Ok(_) => match fs::rename(tmp_path, path)
                .context(format!("Renaming {tmp_path} to {path} failed"))
            {
                Ok(_) => vec![],

                Err(err) => vec![err],
            },
            Err(err) => vec![err],
        };
    if errors.is_empty() {
        return errors;
    }
    if tmp_path.exists() {
        if let Err(err) =
            fs::remove_file(tmp_path).context(format!("{tmp_path} exists and removing it failed"))
        {
            errors.push(err)
        }
    }
    errors
}

fn read_entry(entry: Result<DirEntry, walkdir::Error>) -> AnyhowResult<Section> {
    let entry = entry?;
    let raw = fs::read_to_string(entry.path())?;
    Ok(serde_json::from_str(&raw)?)
}
