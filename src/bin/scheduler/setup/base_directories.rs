use super::fs_entries::{clean_up_file_system_entries, top_level_directories, top_level_files};
use super::ownership::transfer_directory_ownership_recursive;
#[cfg(windows)]
use super::windows_permissions::reset_access;

use crate::internal_config::{GlobalConfig, Plan};

use anyhow::Result as AnyhowResult;
use camino::Utf8Path;
use robotmk::fs::{create_dir_all, remove_dir_all, remove_file};
use robotmk::results::plan_results_directory;
use robotmk::termination::{ContextUnrecoverable, Terminate};

pub fn setup(global_config: &GlobalConfig, plans: &[Plan]) -> Result<(), Terminate> {
    create_dir_all(&global_config.runtime_base_directory)?;
    transfer_directory_ownership_recursive(&global_config.runtime_base_directory)?;
    #[cfg(windows)]
    reset_access(&global_config.runtime_base_directory)?;

    setup_working_directory(global_config, plans)?;
    setup_managed_directory(&global_config.managed_directory)?;
    setup_results_directory(global_config, plans)?;

    Ok(())
}

fn setup_working_directory(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(&global_config.working_directory)?;
    create_dir_all(&global_config.working_directory_plans)?;
    clean_up_file_system_entries(
        plans.iter().map(|plan| &plan.working_directory),
        top_level_directories(&global_config.working_directory_plans)?.iter(),
    )?;
    for dir_to_be_reset in [
        &global_config.working_directory_rcc_setup_steps,
        &global_config.working_directory_environment_building,
    ] {
        if dir_to_be_reset.exists() {
            remove_dir_all(dir_to_be_reset)?;
        }
        create_dir_all(dir_to_be_reset)?;
    }
    Ok(())
}

fn setup_managed_directory(managed_directory: &Utf8Path) -> AnyhowResult<()> {
    if managed_directory.exists() {
        remove_dir_all(managed_directory)?;
    }
    create_dir_all(managed_directory)?;
    Ok(())
}

fn setup_results_directory(global_config: &GlobalConfig, plans: &[Plan]) -> Result<(), Terminate> {
    create_dir_all(&global_config.results_directory)?;
    create_dir_all(plan_results_directory(&global_config.results_directory))?;
    clean_up_results_directory(global_config, plans)
        .context_unrecoverable("Failed to clean up results directory")
}

fn clean_up_results_directory(
    global_config: &GlobalConfig,
    plans: &[Plan],
) -> Result<(), Terminate> {
    let results_directory_lock = global_config
        .results_directory_locker
        .wait_for_write_lock()?;
    for path in top_level_files(&global_config.results_directory)? {
        remove_file(path)?;
    }
    clean_up_file_system_entries(
        plans.iter().map(|plan| &plan.results_file),
        top_level_files(&plan_results_directory(&global_config.results_directory))?.iter(),
    )?;
    Ok(results_directory_lock.release()?)
}
