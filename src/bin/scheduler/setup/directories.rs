use super::plans_by_sessions;
use super::rcc::rcc_setup_working_directory;
use crate::internal_config::{
    environment_building_directory, plans_working_directory, sort_plans_by_grouping, GlobalConfig,
    Plan, Source,
};

use super::api::{self, run_steps, skip, SetupStep, StepWithPlans};
#[cfg(windows)]
use super::windows_permissions::{grant_full_access, reset_access};
use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::environment::Environment;
use robotmk::fs::{create_dir_all, remove_dir_all, remove_file};
use robotmk::results::{plan_results_directory, SetupFailure};
use robotmk::session::Session;
use robotmk::termination::Terminate;
use std::collections::HashSet;

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Terminate> {
    create_dir_all(&global_config.runtime_base_directory)?;
    transfer_directory_ownership_recursive(&global_config.runtime_base_directory)?;
    #[cfg(windows)]
    reset_access(&global_config.runtime_base_directory)?;
    create_dir_all(&global_config.working_directory)?;
    create_dir_all(plans_working_directory(&global_config.working_directory))?;
    for working_sub_dir in [
        rcc_setup_working_directory(&global_config.working_directory),
        environment_building_directory(&global_config.working_directory),
    ] {
        if working_sub_dir.exists() {
            remove_dir_all(&working_sub_dir)?;
        }
        create_dir_all(&working_sub_dir)?;
    }
    clean_up_file_system_entries(
        plans.iter().map(|plan| &plan.working_directory),
        top_level_directories(&plans_working_directory(&global_config.working_directory))?.iter(),
    )?;
    if global_config.managed_directory.exists() {
        remove_dir_all(&global_config.managed_directory)?;
    }
    create_dir_all(&global_config.managed_directory)?;

    setup_results_directories(global_config, &plans)?;

    Ok(run_setup(global_config, plans))
}

fn run_setup(config: &GlobalConfig, mut plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let gather_requirements = [
        gather_managed_directories,
        #[cfg(windows)]
        gather_robocorp_home_base,
        #[cfg(windows)]
        gather_robocorp_home_per_user,
        gather_plan_working_directories,
        gather_environment_building_directories,
        gather_rcc_working_base,
        #[cfg(windows)]
        gather_rcc_longpath_directory,
        gather_rcc_working_per_user,
    ];

    let mut failures = Vec::new();
    for gather in gather_requirements.iter() {
        plans = {
            let plan_count = plans.len();
            let setup_steps = gather(config, plans);
            assert_eq!(
                plan_count,
                setup_steps.iter().map(|s| s.1.len()).sum::<usize>()
            );
            let (surviving_plans, current_errors) = run_steps(setup_steps);
            failures.extend(current_errors);
            surviving_plans
        };
    }
    sort_plans_by_grouping(&mut plans);
    (plans, failures)
}

struct StepCreate {
    target: Utf8PathBuf,
}

impl SetupStep for StepCreate {
    fn setup(&self) -> Result<(), api::Error> {
        create_dir_all(&self.target)
            .map_err(|err| api::Error::new(format!("Failed to create {}", self.target), err))
    }
}

struct StepCreateWithAccess {
    target: Utf8PathBuf,
    session: Session,
}

impl SetupStep for StepCreateWithAccess {
    fn setup(&self) -> Result<(), api::Error> {
        StepCreate {
            target: self.target.clone(),
        }
        .setup()?;
        if let Session::User(user_session) = &self.session {
            log::info!(
                "Granting full access for {} to user `{}`.",
                &self.target,
                &user_session.user_name
            );
            #[cfg(windows)]
            grant_full_access(&user_session.user_name, &self.target).map_err(|err| {
                api::Error::new(
                    format!("Failed to set permissions for {}", self.target),
                    err,
                )
            })?;
        }
        Ok(())
    }
}

#[cfg(windows)]
struct StepRobocorpHomeBase {
    target: Utf8PathBuf,
}

#[cfg(windows)]
impl SetupStep for StepRobocorpHomeBase {
    fn setup(&self) -> Result<(), api::Error> {
        StepCreate {
            target: self.target.clone(),
        }
        .setup()?;
        transfer_directory_ownership_recursive(&self.target).map_err(|err| {
            api::Error::new(
                format!("Failed to transfer ownership of {}", self.target),
                err,
            )
        })?;
        reset_access(&self.target).map_err(|err| {
            api::Error::new(
                format!("Failed to reset permissions of {}", self.target),
                err,
            )
        })?;
        Ok(())
    }
}

#[cfg(windows)]
fn gather_robocorp_home_base(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    vec![
        (
            Box::new(StepRobocorpHomeBase {
                target: config.rcc_config.robocorp_home_base.clone(),
            }),
            rcc_plans,
        ),
        skip(system_plans),
    ]
}

#[cfg(windows)]
fn gather_robocorp_home_per_user(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut setup_steps: Vec<StepWithPlans> = Vec::new();
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        setup_steps.push((
            Box::new(StepCreateWithAccess {
                target: session.robocorp_home(&config.rcc_config.robocorp_home_base),
                session,
            }),
            plans_in_session,
        ));
    }
    setup_steps.push(skip(system_plans));
    setup_steps
}

fn gather_plan_working_directories(
    _global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    plans
        .into_iter()
        .map(|p| {
            (
                Box::new(StepCreateWithAccess {
                    target: p.working_directory.clone(),
                    session: p.session.clone(),
                }) as Box<dyn SetupStep>,
                vec![p],
            )
        })
        .collect()
}

fn gather_environment_building_directories(
    _config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let mut setup_steps: Vec<StepWithPlans> = Vec::new();
    let mut system_plans = Vec::new();
    for plan in plans.into_iter() {
        match &plan.environment {
            Environment::Rcc(rcc_env) => setup_steps.push((
                Box::new(StepCreateWithAccess {
                    target: rcc_env.build_runtime_directory.clone(),
                    session: plan.session.clone(),
                }),
                vec![plan],
            )),
            _ => system_plans.push(plan),
        }
    }
    setup_steps.push(skip(system_plans));
    setup_steps
}

fn gather_rcc_working_base(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    vec![
        (
            Box::new(StepCreate {
                target: rcc_setup_working_directory(&config.working_directory),
            }),
            rcc_plans,
        ),
        skip(system_plans),
    ]
}

fn gather_rcc_working_per_user(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut setup_steps: Vec<StepWithPlans> = Vec::new();
    let base = rcc_setup_working_directory(&config.working_directory);
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        setup_steps.push((
            Box::new(StepCreateWithAccess {
                target: base.join(session.id()),
                session,
            }),
            plans_in_session,
        ));
    }
    setup_steps.push(skip(system_plans));
    setup_steps
}

#[cfg(windows)]
fn gather_rcc_longpath_directory(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    use robotmk::session::CurrentSession;
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    vec![
        (
            Box::new(StepCreate {
                target: rcc_setup_working_directory(&config.working_directory)
                    .join(CurrentSession {}.id()),
            }),
            rcc_plans,
        ),
        skip(system_plans),
    ]
}

fn setup_results_directories(global_config: &GlobalConfig, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(&global_config.results_directory)?;
    create_dir_all(plan_results_directory(&global_config.results_directory))?;
    clean_up_results_directory(global_config, plans).context("Failed to clean up results directory")
}

fn gather_managed_directories(
    _global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let mut setup_steps: Vec<StepWithPlans> = Vec::new();
    let mut unaffected_plans = Vec::new();
    for plan in plans.into_iter() {
        if let Source::Managed { target, .. } = &plan.source {
            setup_steps.push((
                Box::new(StepCreateWithAccess {
                    target: target.clone(),
                    session: plan.session.clone(),
                }),
                vec![plan],
            ));
        } else {
            unaffected_plans.push(plan);
        }
    }
    setup_steps.push(skip(unaffected_plans));
    setup_steps
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

#[cfg(unix)]
pub fn transfer_directory_ownership_recursive(target: &Utf8Path) -> AnyhowResult<()> {
    let user_id = unsafe { libc::getuid() };
    let group_id = unsafe { libc::getgid() };
    let mut targets: Vec<Utf8PathBuf> = vec![target.into()];
    while let Some(target) = targets.pop() {
        std::os::unix::fs::lchown(&target, Some(user_id), Some(group_id)).context(format!(
            "Failed to set ownership of {target} to `{user_id}:{group_id}`",
        ))?;
        if target.is_dir() && !target.is_symlink() {
            targets.extend(top_level_directory_entries(&target)?);
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn transfer_directory_ownership_recursive(target: &Utf8Path) -> AnyhowResult<()> {
    super::windows_permissions::transfer_directory_ownership_to_admin_group_recursive(target)
}

fn top_level_directories(directory: &Utf8Path) -> AnyhowResult<Vec<Utf8PathBuf>> {
    Ok(top_level_directory_entries(directory)?
        .into_iter()
        .filter(|path| path.is_dir())
        .collect())
}

fn top_level_files(directory: &Utf8Path) -> AnyhowResult<Vec<Utf8PathBuf>> {
    Ok(top_level_directory_entries(directory)?
        .into_iter()
        .filter(|path| path.is_file())
        .collect())
}

fn top_level_directory_entries(directory: &Utf8Path) -> AnyhowResult<Vec<Utf8PathBuf>> {
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

fn clean_up_file_system_entries<P>(
    entries_to_keep: impl IntoIterator<Item = P>,
    currently_present_entries: impl IntoIterator<Item = P>,
) -> AnyhowResult<()>
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
