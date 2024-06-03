use super::{failed_plan_ids_human_readable, grant_permissions_to_all_plan_users};
use crate::build::environment_building_working_directory;
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan, Source};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use robotmk::results::{plan_results_directory, GeneralSetupFailures};
use robotmk::section::WriteSection;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, remove_dir_all, remove_file};

pub fn setup(global_config: &GlobalConfig, plans: Vec<Plan>) -> AnyhowResult<Vec<Plan>> {
    setup_working_directories(&global_config.working_directory, &plans)?;
    setup_results_directories(global_config, &plans)?;
    setup_managed_directories(&global_config.managed_directory, &plans)?;

    let mut surviving_plans: Vec<Plan>;
    let mut general_setup_failures = GeneralSetupFailures::default();
    (
        surviving_plans,
        general_setup_failures.working_directory_permissions,
    ) = adjust_working_directory_permissions(global_config, plans);

    general_setup_failures.write(
        global_config
            .results_directory
            .join("general_setup_failures.json"),
        &global_config.results_directory_locker,
    )?;

    sort_plans_by_grouping(&mut surviving_plans);
    Ok(surviving_plans)
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
        .context("Failed to create environment building working directory")
}

fn setup_results_directories(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(&global_config.results_directory)
        .context("Failed to create results directory")?;
    create_dir_all(plan_results_directory(&global_config.results_directory))
        .context("Failed to create plan results directory")?;
    clean_up_results_directory(global_config, plans).context("Failed to clean up results directory")
}

fn setup_managed_directories(managed_directory: &Utf8Path, plans: &[Plan]) -> AnyhowResult<()> {
    if managed_directory.exists() {
        remove_dir_all(managed_directory).context("Failed to remove managed directory")?;
    }
    create_dir_all(managed_directory).context("Failed to create managed directory")?;
    for plan in plans {
        if let Source::Managed { target, .. } = &plan.source {
            create_dir_all(target).context(anyhow!(
                "Failed to create managed directory {} for plan {}",
                target,
                plan.id
            ))?;
        }
    }
    Ok(())
}

fn adjust_working_directory_permissions(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> (Vec<Plan>, HashMap<String, String>) {
    debug!(
        "Granting all plan users full access to {}",
        global_config.working_directory
    );
    let (surviving_plans, failures_by_plan_id) = grant_permissions_to_all_plan_users(
        &global_config.working_directory,
        plans,
        "(OI)(CI)F",
        &["/T"],
    );

    if !failures_by_plan_id.is_empty() {
        error!(
            "Dropping the following plans due to failure to adjust working directory permissions: {}",
            failed_plan_ids_human_readable(failures_by_plan_id.keys())
        );
    }

    (surviving_plans, failures_by_plan_id)
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
