use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::fs::{remove_dir_all, remove_file};
use std::collections::HashSet;

pub fn top_level_directory_entries(directory: &Utf8Path) -> anyhow::Result<Vec<Utf8PathBuf>> {
    let mut entries = vec![];

    for dir_entry in directory
        .read_dir_utf8()
        .context(format!("Failed to read entries of directory {directory}",))?
    {
        entries.push(
            dir_entry
                .context(format!("Failed to read entries of directory {directory}",))?
                .path()
                .to_path_buf(),
        )
    }

    Ok(entries)
}

pub fn top_level_directories(directory: &Utf8Path) -> anyhow::Result<Vec<Utf8PathBuf>> {
    Ok(top_level_directory_entries(directory)?
        .into_iter()
        .filter(|path| path.is_dir())
        .collect())
}

pub fn top_level_files(directory: &Utf8Path) -> anyhow::Result<Vec<Utf8PathBuf>> {
    Ok(top_level_directory_entries(directory)?
        .into_iter()
        .filter(|path| path.is_file())
        .collect())
}

pub fn clean_up_file_system_entries<P>(
    entries_to_keep: impl IntoIterator<Item = P>,
    currently_present_entries: impl IntoIterator<Item = P>,
) -> anyhow::Result<()>
where
    P: AsRef<Utf8Path>,
    P: std::cmp::Eq,
    P: std::hash::Hash,
{
    for entry in HashSet::<P>::from_iter(currently_present_entries)
        .difference(&HashSet::from_iter(entries_to_keep))
    {
        if entry.as_ref().is_file() {
            remove_file(entry)?
        } else {
            remove_dir_all(entry)?
        }
    }
    Ok(())
}
