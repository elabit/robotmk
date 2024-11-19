use super::api::{self, run_steps, skip, SetupStep, StepWithPlans};
#[cfg(windows)]
use super::ownership::transfer_directory_ownership_recursive;
use super::plans_by_sessions;
#[cfg(windows)]
use super::windows_permissions::{grant_full_access, reset_access};

use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan, Source};

use camino::Utf8PathBuf;
use robotmk::environment::Environment;
use robotmk::fs::create_dir_all;
use robotmk::results::SetupFailure;
use robotmk::session::Session;
use robotmk::termination::Cancelled;

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    run_setup_steps(global_config, plans)
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
                target: config.working_directory_rcc_setup_steps.clone(),
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
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        setup_steps.push((
            Box::new(StepCreateWithAccess {
                target: config.working_directory_rcc_setup_steps.join(session.id()),
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
                target: config
                    .working_directory_rcc_setup_steps
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
