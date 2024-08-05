use super::rcc::rcc_setup_working_directory;
use super::{grant_full_access, plans_by_sessions};
use crate::build::environment_building_working_directory;
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan};

use anyhow::{anyhow, Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::info;
use robotmk::environment::Environment;
use robotmk::results::{plan_results_directory, GeneralSetupFailures};
use robotmk::section::WriteSection;
use robotmk::session::Session;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, remove_dir_all, remove_file};

pub fn setup(global_config: &GlobalConfig, plans: Vec<Plan>) -> AnyhowResult<Vec<Plan>> {
    if global_config.working_directory.exists() {
        remove_dir_all(&global_config.working_directory)
            .context("Failed to remove working directory")?;
    }
    create_dir_all(&global_config.working_directory)
        .context("Failed to create working directory")?;
    setup_results_directories(global_config, &plans)?;

    let mut surviving_plans: Vec<Plan> = setup_working_directories(global_config, plans)?;
    sort_plans_by_grouping(&mut surviving_plans);
    Ok(surviving_plans)
}

fn setup_working_directories(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> AnyhowResult<Vec<Plan>> {
    let (surviving_plans, plan_failures) = setup_plans_working_directory(plans);
    let (surviving_plans, rcc_failures) =
        setup_rcc_working_directories(&global_config.working_directory, surviving_plans);
    GeneralSetupFailures {
        working_directory_permissions: plan_failures
            .into_iter()
            .chain(rcc_failures.into_iter())
            .collect(),
    }
    .write(
        global_config
            .results_directory
            .join("general_setup_failures.json"),
        &global_config.results_directory_locker,
    )?;
    Ok(surviving_plans)
}

fn setup_plans_working_directory(plans: Vec<Plan>) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = Vec::new();
    let mut failures = HashMap::new();
    for plan in plans.into_iter() {
        if let Err(e) = create_dir_all(&plan.working_directory) {
            let error = anyhow!(e).context(format!(
                "Failed to create working directory {} of plan {}",
                plan.working_directory, plan.id
            ));
            info!("{error:#}");
            failures.insert(plan.id.clone(), format!("{error:#}"));
            continue;
        }
        if let Session::User(user_session) = &plan.session {
            info!(
                "Granting full access for {} to user `{}`.",
                &plan.working_directory, &user_session.user_name
            );
            if let Err(e) = grant_full_access(&user_session.user_name, &plan.working_directory) {
                let error = anyhow!(e).context(format!(
                    "Failed to set permissions for working directory {} of plan {}",
                    plan.working_directory, plan.id
                ));
                info!("{error:#}");
                failures.insert(plan.id.clone(), format!("{error:#}"));
                continue;
            };
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

fn setup_rcc_working_directories(
    working_directory: &Utf8Path,
    plans: Vec<Plan>,
) -> (Vec<Plan>, HashMap<String, String>) {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let (surviving_plans, environment_failures) = setup_with_one_directory_per_user(
        &environment_building_working_directory(working_directory),
        rcc_plans,
    );
    let (mut surviving_plans, rcc_setup_failures) = setup_with_one_directory_per_user(
        &rcc_setup_working_directory(working_directory),
        surviving_plans,
    );
    surviving_plans.extend(system_plans);
    (
        surviving_plans,
        environment_failures
            .into_iter()
            .chain(rcc_setup_failures)
            .collect(),
    )
}

fn setup_with_one_directory_per_user(
    target: &Utf8Path,
    plans: Vec<Plan>,
) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = Vec::new();
    let mut failures = HashMap::new();
    if let Err(e) = create_dir_all(target) {
        let error = anyhow!(e).context(format!("Failed to create directory {target}",));
        info!("{error:#}");
        for plan in plans {
            failures.insert(plan.id.clone(), format!("{error:#}"));
        }
        return (surviving_plans, failures);
    }
    for (session, plans_in_session) in plans_by_sessions(plans) {
        let user_target = &target.join(session.id());
        if let Err(e) = create_dir_all(user_target) {
            let error = anyhow!(e).context(format!(
                "Failed to create directory {} for session {}",
                user_target, &session
            ));
            info!("{error:#}");
            for plan in plans_in_session {
                failures.insert(plan.id.clone(), format!("{error:#}"));
            }
            continue;
        }
        if let Session::User(user_session) = &session {
            info!(
                "Granting full access for {} to user `{}`.",
                user_target, &user_session.user_name
            );
            if let Err(e) = grant_full_access(&user_session.user_name, user_target) {
                let error = anyhow!(e).context(format!(
                    "Failed to grant full access for {} to user `{}`.",
                    user_target, &user_session.user_name
                ));
                info!("{error:#}");
                for plan in plans_in_session {
                    failures.insert(plan.id.clone(), format!("{error:#}"));
                }
                continue;
            };
        }
        surviving_plans.extend(plans_in_session);
    }
    (surviving_plans, failures)
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
