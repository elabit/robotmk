use super::api::{self, run_steps, skip, SetupStep, StepWithPlans};
use super::fs_entries::{clean_up_file_system_entries, top_level_directories, top_level_files};
use super::ownership::transfer_directory_ownership_recursive;
use super::plans_by_sessions;
use super::rcc::rcc_setup_working_directory;
#[cfg(windows)]
use super::windows_permissions::{grant_full_access, reset_access};

use crate::internal_config::{
    environment_building_directory, plans_working_directory, sort_plans_by_grouping, GlobalConfig,
    Plan, Source,
};

use anyhow::Result as AnyhowResult;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::environment::Environment;
use robotmk::fs::{create_dir_all, remove_dir_all, remove_file};
use robotmk::results::{plan_results_directory, SetupFailure};
use robotmk::session::Session;
use robotmk::termination::{Cancelled, ContextUnrecoverable, Terminate};

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Terminate> {
    create_dir_all(&global_config.runtime_base_directory)?;
    transfer_directory_ownership_recursive(&global_config.runtime_base_directory)?;
    #[cfg(windows)]
    reset_access(&global_config.runtime_base_directory)?;

    setup_working_directory(&global_config.working_directory, &plans)?;
    setup_managed_directory(&global_config.managed_directory)?;
    setup_results_directory(global_config, &plans)?;

    Ok(run_setup_steps(global_config, plans)?)
}

fn setup_working_directory(working_directory: &Utf8Path, plans: &[Plan]) -> AnyhowResult<()> {
    create_dir_all(working_directory)?;
    create_dir_all(plans_working_directory(working_directory))?;
    clean_up_file_system_entries(
        plans.iter().map(|plan| &plan.working_directory),
        top_level_directories(&plans_working_directory(working_directory))?.iter(),
    )?;
    for dir_to_be_reset in [
        rcc_setup_working_directory(working_directory),
        environment_building_directory(working_directory),
    ] {
        if dir_to_be_reset.exists() {
            remove_dir_all(&dir_to_be_reset)?;
        }
        create_dir_all(&dir_to_be_reset)?;
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

fn run_setup_steps(
    config: &GlobalConfig,
    mut plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
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
            let (surviving_plans, current_errors) =
                run_steps(setup_steps, &config.cancellation_token)?;
            failures.extend(current_errors);
            surviving_plans
        };
    }
    sort_plans_by_grouping(&mut plans);
    Ok((plans, failures))
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
