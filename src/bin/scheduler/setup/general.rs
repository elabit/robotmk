use super::all_configured_users;
use super::icacls::run_icacls_command;
use crate::build::environment_building_working_directory;
use crate::internal_config::{GlobalConfig, Plan};
use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::debug;
use robotmk::results::plan_results_directory;
use std::collections::HashSet;
use std::fs::{create_dir_all, remove_file};

pub fn setup(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    setup_working_directories(&global_config.working_directory, plans)?;
    setup_results_directories(global_config, plans)
}

fn setup_working_directories(working_directory: &Utf8Path, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(working_directory).context("Failed to create working directory")?;
    for plan in plans {
        create_dir_all(&plan.working_directory).context(format!(
            "Failed to create working directory {} of plan {}",
            plan.working_directory, plan.id
        ))?;
    }
    create_dir_all(environment_building_working_directory(working_directory))
        .context("Failed to create environment building working directory")?;

    for user_name in all_configured_users(plans.iter()) {
        adjust_working_directory_permissions(working_directory, user_name)
            .context("Failed adjust working directory permissions")?;
    }
    Ok(())
}

fn setup_results_directories(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(&global_config.results_directory)
        .context("Failed to create results directory")?;
    create_dir_all(plan_results_directory(&global_config.results_directory))
        .context("Failed to create plan results directory")?;
    clean_up_results_directory(global_config, plans).context("Failed to clean up results directory")
}

fn clean_up_results_directory(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    let results_directory_lock = global_config
        .results_directory_locker
        .wait_for_write_lock()?;
    for path in top_level_files(&global_config.results_directory)? {
        remove_file(path)?;
    }
    clean_up_plan_results_directory(
        &plan_results_directory(&global_config.results_directory),
        plans,
    )?;
    results_directory_lock.release()
}

fn clean_up_plan_results_directory(
    plan_results_directory: &Utf8Path,
    plans: &[Plan],
) -> AnyhowResult<()> {
    let result_files_to_keep =
        HashSet::<Utf8PathBuf>::from_iter(plans.iter().map(|plan| plan.results_file.clone()));
    let currently_present_result_files =
        HashSet::<Utf8PathBuf>::from_iter(top_level_files(plan_results_directory)?);
    for path in currently_present_result_files.difference(&result_files_to_keep) {
        remove_file(path)?;
    }
    Ok(())
}

fn top_level_files(directory: &Utf8Path) -> AnyhowResult<Vec<Utf8PathBuf>> {
    let mut result_files = vec![];

    for dir_entry in directory.read_dir_utf8().context(format!(
        "Failed to read entries of results directory {directory}",
    ))? {
        let dir_entry = dir_entry.context(format!(
            "Failed to read entries of results directory {directory}",
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

fn adjust_working_directory_permissions(
    working_directory: &Utf8Path,
    user_name: &str,
) -> AnyhowResult<()> {
    debug!("Granting user `{user_name}` full access to {working_directory}");
    run_icacls_command(vec![
        working_directory.as_str(),
        "/grant",
        &format!("{user_name}:(OI)(CI)F"),
        "/T",
    ])
    .context(format!(
        "Adjusting permissions of {working_directory} for user `{user_name}` failed"
    ))
}
