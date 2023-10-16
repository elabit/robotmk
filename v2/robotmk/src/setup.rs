use super::config::internal::{GlobalConfig, Suite};
use super::environment::{environment_building_stdio_directory, Environment};
use super::results::suite_results_directory;

use anyhow::{bail, Context, Result};
use atomicwrites::replace_atomic;
use camino::{Utf8Path, Utf8PathBuf};
use log::debug;
use std::collections::HashSet;
use std::fs::{create_dir_all, remove_file};
use std::process::Command;

pub fn setup(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    create_dir_all(&global_config.working_directory)
        .context("Failed to create working directory")?;
    create_dir_all(environment_building_stdio_directory(
        &global_config.working_directory,
    ))
    .context("Failed to create environment building stdio directory")?;
    create_dir_all(&global_config.results_directory)
        .context("Failed to create results directory")?;
    create_dir_all(suite_results_directory(&global_config.results_directory))
        .context("Failed to create suite results directory")?;
    clean_up_results_directory_atomic(global_config, suites)?;
    adjust_user_permissions(global_config, suites)
}

fn clean_up_results_directory_atomic(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    let suite_results_directory = suite_results_directory(&global_config.results_directory);
    let result_files_to_keep =
        HashSet::<Utf8PathBuf>::from_iter(suites.iter().map(|suite| suite.results_file.clone()));
    let currently_present_result_files = HashSet::<Utf8PathBuf>::from_iter(
        currently_present_result_files(&suite_results_directory)?,
    );
    remove_files_atomic(
        &global_config.working_directory.join("deprecated_result"),
        currently_present_result_files.difference(&result_files_to_keep),
    )
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

fn remove_files_atomic<'a>(
    intermediate_path_for_move: &Utf8Path,
    files_to_be_removed: impl Iterator<Item = &'a Utf8PathBuf>,
) -> Result<()> {
    for path in files_to_be_removed {
        replace_atomic(path.as_std_path(), intermediate_path_for_move.as_std_path()).context(
            format!("Failed to move {path} to {intermediate_path_for_move}",),
        )?;
    }

    let _ = remove_file(intermediate_path_for_move);

    Ok(())
}

fn adjust_user_permissions(global_config: &GlobalConfig, suites: &[Suite]) -> Result<()> {
    adjust_working_directory_permissions(&global_config.working_directory)?;
    for suite in suites {
        if let Environment::Rcc(rcc_environment) = &suite.environment {
            adjust_executable_permissions(&rcc_environment.binary_path)?
        }
    }
    Ok(())
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

fn adjust_executable_permissions(executable_path: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` read and execute access to {executable_path}");
    run_icacls_command(vec![executable_path.as_str(), "/grant", "Users:(RX)"]).context(format!(
        "Adjusting permissions of {executable_path} for group `Users` failed",
    ))
}

fn run_icacls_command<'a>(arguments: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut command = Command::new("icacls.exe");
    command.args(arguments);
    let output = command
        .output()
        .context(format!("Calling icacls.exe failed. Command:\n{command:?}"))?;
    if !output.status.success() {
        bail!(
            "icacls.exe exited non-successfully.\n\nCommand:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    Ok(())
}
