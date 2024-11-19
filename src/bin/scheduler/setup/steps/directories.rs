use super::api::{self, skip, SetupStep, StepWithPlans};
use super::{
    partition_into_rcc_and_system_plans, plans_by_sessions, rcc_working_directory_for_session,
};

use crate::internal_config::{GlobalConfig, Plan, Source};
#[cfg(windows)]
use crate::setup::ownership::transfer_directory_ownership_recursive;
#[cfg(windows)]
use crate::setup::windows_permissions::{grant_full_access, reset_access, run_icacls_command};

use camino::Utf8PathBuf;
use robotmk::environment::Environment;
use robotmk::fs::create_dir_all;
use robotmk::session::Session;

struct StepCreate {
    target: Utf8PathBuf,
}

impl SetupStep for StepCreate {
    fn label(&self) -> String {
        format!("Create directory {}", self.target)
    }

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
    fn label(&self) -> String {
        let mut label = StepCreate {
            target: self.target.clone(),
        }
        .label();
        if let Session::User(user_session) = &self.session {
            label = format!(
                "{label} and grant user {user} full access",
                user = user_session.user_name
            );
        }
        label
    }

    fn setup(&self) -> Result<(), api::Error> {
        StepCreate {
            target: self.target.clone(),
        }
        .setup()?;
        #[cfg(windows)]
        if let Session::User(user_session) = &self.session {
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
    fn label(&self) -> String {
        format!(
            "Create ROBOCORP_HOME base directory {} and restrict to Administrator group only",
            self.target
        )
    }

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
        run_icacls_command([self.target.as_str(), "/inheritancelevel:r"]).map_err(|err| {
            api::Error::new(
                format!(
                    "Failed to set remove permission inheritance for {}",
                    self.target
                ),
                err,
            )
        })?;
        grant_full_access(
            "*S-1-5-32-544", // Administrators (https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/understand-security-identifiers)
            &self.target,
        )
        .map_err(|err| {
            api::Error::new(
                format!("Failed to set permissions for {}", self.target),
                err,
            )
        })?;
        Ok(())
    }
}

#[cfg(windows)]
pub fn gather_robocorp_home_base(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
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
pub fn gather_robocorp_home_per_user(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut setup_steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        setup_steps.push((
            Box::new(StepCreateWithAccess {
                target: session.robocorp_home(&config.rcc_config.robocorp_home_base),
                session,
            }),
            plans_in_session,
        ));
    }
    setup_steps
}

pub fn gather_plan_working_directories(
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

pub fn gather_environment_building_directories(
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

pub fn gather_rcc_working_base(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
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

pub fn gather_rcc_working_per_user(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut setup_steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        setup_steps.push((
            Box::new(StepCreateWithAccess {
                target: rcc_working_directory_for_session(
                    &config.working_directory_rcc_setup_steps,
                    &session,
                ),
                session,
            }),
            plans_in_session,
        ));
    }
    setup_steps
}

#[cfg(windows)]
pub fn gather_rcc_longpath_directory(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    use robotmk::session::CurrentSession;
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    vec![
        (
            Box::new(StepCreate {
                target: rcc_working_directory_for_session(
                    &config.working_directory_rcc_setup_steps,
                    &Session::Current(CurrentSession {}),
                ),
            }),
            rcc_plans,
        ),
        skip(system_plans),
    ]
}

pub fn gather_managed_directories(
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
