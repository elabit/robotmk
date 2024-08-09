use super::plans_by_sessions;
use super::rcc::rcc_setup_working_directory;
use crate::build::environment_building_working_directory;
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan, Source};

use anyhow::{anyhow, Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::error;
use robotmk::environment::Environment;
use robotmk::fs::{create_dir_all, remove_dir_all, remove_file};
use robotmk::results::{plan_results_directory, SetupFailure};
use robotmk::termination::Terminate;
use std::collections::HashSet;

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Terminate> {
    if global_config.working_directory.exists() {
        remove_dir_all(&global_config.working_directory)?;
    }
    create_dir_all(&global_config.working_directory)?;
    if global_config.managed_directory.exists() {
        remove_dir_all(&global_config.managed_directory)?;
    }
    create_dir_all(&global_config.managed_directory)?;
    setup_results_directories(global_config, &plans)?;

    let (surviving_plans, managed_dir_failures) = setup_managed_directories(plans);
    let (mut surviving_plans, working_dir_failures) =
        setup_working_directories(global_config, surviving_plans);

    sort_plans_by_grouping(&mut surviving_plans);
    Ok((
        surviving_plans,
        managed_dir_failures
            .into_iter()
            .chain(working_dir_failures)
            .collect(),
    ))
}

fn setup_working_directories(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    let (surviving_plans, plan_failures) = setup_plans_working_directory(plans);
    let (surviving_plans, rcc_failures) =
        setup_rcc_working_directories(&global_config.working_directory, surviving_plans);
    (
        surviving_plans,
        plan_failures.into_iter().chain(rcc_failures).collect(),
    )
}

fn setup_plans_working_directory(plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = Vec::new();
    let mut failures = vec![];
    for plan in plans.into_iter() {
        if let Err(e) = create_dir_all(&plan.working_directory) {
            let error = anyhow!(e);
            error!(
                "Plan {}: Failed to create working directory. Plan won't be scheduled.
                 Error: {error:?}",
                plan.id
            );
            failures.push(SetupFailure {
                plan_id: plan.id.clone(),
                summary: "Failed to create working directory".to_string(),
                details: format!("{error:?}"),
            });
            continue;
        }
        #[cfg(windows)]
        {
            use super::windows_permissions::grant_full_access;
            use log::info;
            use robotmk::session::Session;

            if let Session::User(user_session) = &plan.session {
                info!(
                    "Granting full access for {} to user `{}`.",
                    &plan.working_directory, &user_session.user_name
                );
                if let Err(e) = grant_full_access(&user_session.user_name, &plan.working_directory)
                {
                    let error = anyhow!(e);
                    error!(
                        "Plan {}: Failed to set permissions for working directory. \
                         Plan won't be scheduled.
                         Error: {error:?}",
                        plan.id
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Failed to set permissions for working directory".to_string(),
                        details: format!("{error:?}"),
                    });
                    continue;
                };
            }
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

fn setup_rcc_working_directories(
    working_directory: &Utf8Path,
    plans: Vec<Plan>,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let (surviving_plans, environment_failures) = setup_with_one_directory_per_user(
        &environment_building_working_directory(working_directory),
        rcc_plans,
        "environment building",
    );

    #[cfg(unix)]
    let (mut surviving_plans, rcc_setup_failures) = setup_with_one_directory_per_user(
        &rcc_setup_working_directory(working_directory),
        surviving_plans,
        "RCC setup",
    );
    #[cfg(windows)]
    let (mut surviving_plans, rcc_setup_failures) = {
        let (surviving_plans, rcc_setup_failures) = setup_with_one_directory_per_user(
            &rcc_setup_working_directory(working_directory),
            surviving_plans,
            "RCC setup",
        );
        let (surviving_plans, rcc_setup_failures_long_path_support) =
            setup_with_one_directory_for_current_session(
                &rcc_setup_working_directory(working_directory),
                surviving_plans,
                "RCC setup (long path support)",
            );
        (
            surviving_plans,
            [rcc_setup_failures, rcc_setup_failures_long_path_support].concat(),
        )
    };

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
    description_for_failure_reporting: &str,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = Vec::new();
    let mut failures = vec![];
    if let Err(e) = create_dir_all(target) {
        let error = anyhow!(e);
        for plan in plans {
            error!(
                "Plan {}: Failed to create {description_for_failure_reporting} directory. \
                 Plan won't be scheduled.
                 Error: {error:?}",
                plan.id
            );
            failures.push(SetupFailure {
                plan_id: plan.id.clone(),
                summary: format!("Failed to create {description_for_failure_reporting} directory"),
                details: format!("{error:?}"),
            });
        }
        return (surviving_plans, failures);
    }
    for (session, plans_in_session) in plans_by_sessions(plans) {
        let user_target = &target.join(session.id());
        if let Err(e) = create_dir_all(user_target) {
            let error = anyhow!(e);
            for plan in plans_in_session {
                error!(
                    "Plan {}: Failed to create user-specific {description_for_failure_reporting} \
                     directory. Plan won't be scheduled.
                     Error: {error:?}",
                    plan.id
                );
                failures.push(SetupFailure {
                    plan_id: plan.id.clone(),
                    summary: format!("Failed to create user-specific {description_for_failure_reporting} directory"),
                    details: format!("{error:?}"),
                });
            }
            continue;
        }
        #[cfg(windows)]
        {
            use super::windows_permissions::grant_full_access;
            use log::info;
            use robotmk::session::Session;

            if let Session::User(user_session) = &session {
                info!(
                    "Granting full access for {} to user `{}`.",
                    user_target, &user_session.user_name
                );
                if let Err(e) = grant_full_access(&user_session.user_name, user_target) {
                    let error = anyhow!(e);
                    for plan in plans_in_session {
                        error!(
                            "Plan {}: Failed to adjust permissions for user-specific \
                             {description_for_failure_reporting} directory. Plan won't be scheduled.
                             Error: {error:?}",
                            plan.id
                        );
                        failures.push(SetupFailure {
                            plan_id: plan.id.clone(),
                            summary: format!("Failed to adjust permissions for user-specific {description_for_failure_reporting} directory"),
                            details: format!("{error:?}"),
                        });
                    }
                    continue;
                };
            }
        }
        surviving_plans.extend(plans_in_session);
    }
    (surviving_plans, failures)
}

#[cfg(windows)]
fn setup_with_one_directory_for_current_session(
    target: &Utf8Path,
    plans: Vec<Plan>,
    description_for_failure_reporting: &str,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    use robotmk::session::CurrentSession;

    match create_dir_all(target.join(CurrentSession {}.id())) {
        Ok(()) => (plans, vec![]),
        Err(error) => {
            let mut failures = vec![];
            for plan in plans {
                error!(
                    "Plan {}: Failed to create {description_for_failure_reporting} directory. \
                     Plan won't be scheduled.
                     Error: {error:?}",
                    plan.id
                );
                failures.push(SetupFailure {
                    plan_id: plan.id.clone(),
                    summary: format!(
                        "Failed to create {description_for_failure_reporting} directory"
                    ),
                    details: format!("{error:?}"),
                });
            }
            (vec![], failures)
        }
    }
}

fn setup_results_directories(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(&global_config.results_directory)?;
    create_dir_all(plan_results_directory(&global_config.results_directory))?;
    clean_up_results_directory(global_config, plans).context("Failed to clean up results directory")
}

fn setup_managed_directories(plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = Vec::new();
    let mut failures = vec![];
    for plan in plans {
        if let Source::Managed { target, .. } = &plan.source {
            if let Err(e) = create_dir_all(target) {
                let error = anyhow!(e);
                error!(
                    "Plan {}: Failed to create managed directory. Plan won't be scheduled.
                     Error: {error:?}",
                    plan.id
                );
                failures.push(SetupFailure {
                    plan_id: plan.id.clone(),
                    summary: "Failed to create managed directory".to_string(),
                    details: format!("{error:?}"),
                });
                continue;
            }
            #[cfg(windows)]
            {
                use super::windows_permissions::grant_full_access;
                use log::info;
                use robotmk::session::Session;

                if let Session::User(user_session) = &plan.session {
                    if let Err(error) = grant_full_access(&user_session.user_name, target) {
                        error!(
                            "Plan {}: Failed to adjust permissions of managed directory. Plan won't be scheduled.
                             Error: {error:?}",
                            plan.id
                        );
                        failures.push(SetupFailure {
                            plan_id: plan.id.clone(),
                            summary: "Failed to adjust permissions of managed directory"
                                .to_string(),
                            details: format!("{error:?}"),
                        });
                        continue;
                    }
                    info!(
                        "Adjusted permissions for {} for user `{}`.",
                        target, &user_session.user_name
                    );
                }
            }
        }
        surviving_plans.push(plan)
    }
    (surviving_plans, failures)
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
    clean_up_plan_results_directory(
        &plan_results_directory(&global_config.results_directory),
        plans,
    )?;
    Ok(results_directory_lock.release()?)
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
