use crate::internal_config::Plan;
use crate::log_and_return_error;
use robotmk::config::WorkingDirectoryCleanupConfig;

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8DirEntry, Utf8Path};
use log::{debug, info};
use std::cmp::min;
use std::fs::{remove_dir_all, remove_file};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn cleanup_working_directories<'a>(plans: impl Iterator<Item = &'a Plan>) {
    for plan in plans {
        info!(
            "Cleaning up working directory {} of plan {}",
            plan.working_directory, plan.id
        );
        let _ = cleanup_working_directory(
            &plan.working_directory,
            &plan.working_directory_cleanup_config,
        )
        .context(format!(
            "Error while cleaning up working directory of plan {}",
            plan.id
        ))
        .map_err(log_and_return_error);
    }
}

fn cleanup_working_directory(
    directory: &Utf8Path,
    cleanup_config: &WorkingDirectoryCleanupConfig,
) -> AnyhowResult<()> {
    let dir_entries = directory
        .read_dir_utf8()?
        .filter_map(|dir_entry_result| {
            dir_entry_result
                .context(format!("Failed to list entry of directory {directory}"))
                .map_err(log_and_return_error)
                .ok()
        })
        .collect::<Vec<_>>();
    for dir_entry in match cleanup_config {
        WorkingDirectoryCleanupConfig::MaxAgeSecs(max_age_secs) => {
            dir_entries_to_remove_max_age(dir_entries, *max_age_secs)
        }
        WorkingDirectoryCleanupConfig::MaxExecutions(max_executions) => {
            dir_entries_to_remove_max_executions(dir_entries, *max_executions)
        }
    } {
        let _ = remove_dir_entry(&dir_entry).map_err(log_and_return_error);
    }
    Ok(())
}

fn dir_entries_to_remove_max_age(
    dir_entries: Vec<Utf8DirEntry>,
    max_age_secs: u64,
) -> Vec<Utf8DirEntry> {
    let now = SystemTime::now();
    dir_entries
        .into_iter()
        .filter(|dir_entry| {
            is_dir_entry_too_old(dir_entry, max_age_secs, &now)
                .context("Failure while checking if directory entry should be removed")
                .map_err(log_and_return_error)
                .unwrap_or(false)
        })
        .collect()
}

fn is_dir_entry_too_old(
    dir_entry: &Utf8DirEntry,
    max_age_secs: u64,
    now: &SystemTime,
) -> AnyhowResult<bool> {
    match now.duration_since(
        dir_entry
            .metadata()
            .context(format!(
                "Failed to retrieve metadata of {}",
                dir_entry.path()
            ))?
            .modified()
            .context(format!(
                "Failed to retrieve modification time of {}",
                dir_entry.path()
            ))?,
    ) {
        Ok(duration) => Ok(duration.as_secs() > max_age_secs),
        // now is earlier than modification time --> directory was modified in the meantime, so keep
        Err(_) => Ok(false),
    }
}

fn dir_entries_to_remove_max_executions(
    dir_entries: Vec<Utf8DirEntry>,
    max_executions: usize,
) -> Vec<Utf8DirEntry> {
    let mut dir_entries_sorted_by_mtime = sort_dir_entries_by_mtime(dir_entries);
    dir_entries_sorted_by_mtime.reverse();
    split_vec(dir_entries_sorted_by_mtime, max_executions).1
}

fn sort_dir_entries_by_mtime(dir_entries: Vec<Utf8DirEntry>) -> Vec<Utf8DirEntry> {
    let mut dir_entries_with_mtime: Vec<(Utf8DirEntry, u64)> = dir_entries
        .into_iter()
        .filter_map(|dir_entry| {
            dir_entry
                .metadata()
                .context(format!(
                    "Failed to retrieve metadata of {}",
                    dir_entry.path()
                ))
                .map_err(log_and_return_error)
                .ok()
                .map(|metadata| (dir_entry, metadata))
        })
        .filter_map(|(dir_entry, metatada)| {
            metatada
                .modified()
                .context(format!(
                    "Failed to retrieve modification time of {}",
                    dir_entry.path()
                ))
                .map_err(log_and_return_error)
                .ok()
                .map(|mtime| (dir_entry, mtime))
        })
        .filter_map(|(dir_entry, mtime)| {
            mtime
                .duration_since(UNIX_EPOCH)
                .context(format!(
                    "Failed to compute modification time of {} as Unix timestamp",
                    dir_entry.path()
                ))
                .map_err(log_and_return_error)
                .ok()
                .map(|duration| (dir_entry, duration.as_secs()))
        })
        .collect();
    dir_entries_with_mtime.sort_by_key(|(_dir_entry, mtime)| *mtime);
    dir_entries_with_mtime
        .into_iter()
        .map(|(dir_entry, _mtime)| dir_entry)
        .collect()
}

fn split_vec<T>(mut vector: Vec<T>, at: usize) -> (Vec<T>, Vec<T>) {
    let tail = vector.split_off(min(at, vector.len()));
    (vector, tail)
}

fn remove_dir_entry(dir_entry: &Utf8DirEntry) -> AnyhowResult<()> {
    debug!("Removing {}", dir_entry.path());
    (if dir_entry
        .file_type()
        .context(format!(
            "Failed to determine file type of {}",
            dir_entry.path()
        ))?
        .is_dir()
    {
        remove_dir_all
    } else {
        remove_file
    })(dir_entry.path())
    .context(format!("Failed to remove {}", dir_entry.path()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_vec_at_zero() {
        assert_eq!(split_vec(vec![1, 2, 3], 0), (vec![], vec![1, 2, 3]))
    }

    #[test]
    fn split_vec_in_between() {
        assert_eq!(split_vec(vec![1, 2, 3], 2), (vec![1, 2], vec![3]))
    }

    #[test]
    fn split_vec_at_len() {
        assert_eq!(split_vec(vec![1, 2, 3], 3), (vec![1, 2, 3], vec![]))
    }

    #[test]
    fn split_vec_at_larger_than_len() {
        assert_eq!(split_vec(vec![1, 2, 3], 4), (vec![1, 2, 3], vec![]))
    }
}
