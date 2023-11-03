use super::icacls::run_icacls_command;
use crate::environment::environment_building_stdio_directory;
use crate::internal_config::{GlobalConfig, Suite};
use crate::results::suite_results_directory;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::debug;
use std::collections::HashSet;
use std::fs::{create_dir_all, remove_file};

pub fn setup(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    setup_working_directories(&global_config.working_directory, suites)?;
    setup_results_directories(global_config, suites)
}

fn setup_working_directories(working_directory: &Utf8Path, suites: &[Suite]) -> Result<()> {
    create_dir_all(working_directory).context("Failed to create working directory")?;
    for suite in suites {
        create_dir_all(&suite.working_directory).context(format!(
            "Failed to create working directory {} of suite {}",
            suite.working_directory, suite.name
        ))?;
    }
    create_dir_all(environment_building_stdio_directory(working_directory))
        .context("Failed to create environment building stdio directory")?;
    adjust_working_directory_permissions(working_directory)
        .context("Failed adjust working directory permissions")
}

fn setup_results_directories(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    create_dir_all(&global_config.results_directory)
        .context("Failed to create results directory")?;
    create_dir_all(suite_results_directory(&global_config.results_directory))
        .context("Failed to create suite results directory")?;
    clean_up_results_directory_atomic(global_config, suites)
        .context("Failed to clean up results directory")
}

fn clean_up_results_directory_atomic(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    let suite_results_directory = suite_results_directory(&global_config.results_directory);
    let result_files_to_keep =
        HashSet::<Utf8PathBuf>::from_iter(suites.iter().map(|suite| suite.results_file.clone()));
    let currently_present_result_files = HashSet::<Utf8PathBuf>::from_iter(
        currently_present_result_files(&suite_results_directory)?,
    );
    for path in currently_present_result_files.difference(&result_files_to_keep) {
        remove_file(path)?; // TODO: This fails, if the agent plugin is currently reading the file
                            // (a non-critical and recoverable) error. How to handle it?
    }
    Ok(())
}

fn currently_present_result_files(suite_results_directory: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let mut result_files = vec![];

    for dir_entry in suite_results_directory.read_dir_utf8().context(format!(
        "Failed to read entries of results directory {suite_results_directory}",
    ))? {
        let dir_entry = dir_entry.context(format!(
            "Failed to read entries of results directory {suite_results_directory}",
        ))?;
        if dir_entry
            .file_type()
            .context(format!(
                "Failed to determine file type of {}",
                dir_entry.path()
            ))?
            .is_file()
        {
            result_files.push(dir_entry.path().to_path_buf())
        }
    }

    Ok(result_files)
}

fn adjust_working_directory_permissions(working_directory: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` full access to {working_directory}");
    run_icacls_command(vec![
        working_directory.as_str(),
        "/grant",
        "Users:(OI)(CI)F",
        "/T",
    ])
    .context(format!(
        "Adjusting permissions of {working_directory} for group `Users` failed"
    ))
}
