use super::api::{self, skip, SetupStep, StepWithPlans};
use super::{
    partition_into_rcc_and_system_plans, plans_by_sessions, rcc_working_directory_for_session,
};

use crate::internal_config::{GlobalConfig, Plan};
use crate::logging::log_and_return_error;
#[cfg(windows)]
use crate::setup::windows_permissions::run_icacls_command;

use robotmk::config::RCCProfileConfig;
use robotmk::environment::RCCEnvironment;
#[cfg(windows)]
use robotmk::session::CurrentSession;
use robotmk::session::{RunSpec, Session};
use robotmk::termination::Outcome;

use anyhow::{anyhow, Context};
use camino::Utf8PathBuf;
use log::{debug, error};
use std::vec;
use tokio_util::sync::CancellationToken;

#[cfg(windows)]
struct StepFilePermissions {
    target: Utf8PathBuf,
    session: Session,
    icacls_permissions: String,
}

#[cfg(windows)]
impl SetupStep for StepFilePermissions {
    fn setup(&self) -> Result<(), api::Error> {
        if let Session::User(user_session) = &self.session {
            log::info!(
                "Granting user `{user}` {permissions} access to {target}.",
                user = &user_session.user_name,
                permissions = &self.icacls_permissions,
                target = &self.target,
            );
            run_icacls_command([
                self.target.as_str(),
                "/grant",
                &format!("{}:{}", &user_session.user_name, self.icacls_permissions),
            ])
            .map_err(|err| {
                api::Error::new(
                    format!(
                        "Adjusting permissions of {} for user `{}` failed",
                        self.target, &user_session.user_name
                    ),
                    err,
                )
            })?;
        }
        Ok(())
    }
}

struct StepRCCCommand {
    binary_path: Utf8PathBuf,
    robocorp_home_base: Utf8PathBuf,
    working_directory: Utf8PathBuf,
    session: Session,
    cancellation_token: CancellationToken,
    arguments: Vec<String>,
    id: String,
    summary_if_failure: String,
}

impl StepRCCCommand {
    fn new_from_config(
        config: &GlobalConfig,
        session: Session,
        arguments: &[&str],
        id: &str,
        summary_if_failure: &str,
    ) -> Self {
        Self {
            binary_path: config.rcc_config.binary_path.clone(),
            robocorp_home_base: config.rcc_config.robocorp_home_base.clone(),
            working_directory: config.working_directory_rcc_setup_steps.clone(),
            session,
            cancellation_token: config.cancellation_token.clone(),
            arguments: arguments.iter().map(|s| s.to_string()).collect(),
            id: id.into(),
            summary_if_failure: summary_if_failure.into(),
        }
    }
}

impl SetupStep for StepRCCCommand {
    fn setup(&self) -> Result<(), api::Error> {
        let mut command_spec = RCCEnvironment::bundled_command_spec(
            &self.binary_path,
            self.session
                .robocorp_home(&self.robocorp_home_base)
                .to_string(),
        );
        command_spec.add_arguments(&self.arguments);
        let run_spec = RunSpec {
            id: &format!("robotmk_{}", self.id),
            command_spec: &command_spec,
            runtime_base_path: &rcc_working_directory_for_session(
                &self.working_directory,
                &self.session,
            )
            .join(&self.id),
            timeout: 120,
            cancellation_token: &self.cancellation_token,
        };
        debug!("Running {} for `{}`", command_spec, &self.session);
        let run_outcome = match self.session.run(&run_spec).context(format!(
            "Failed to run {} for `{}`",
            run_spec.command_spec, self.session
        )) {
            Ok(run_outcome) => run_outcome,
            Err(error) => {
                let error = log_and_return_error(error);
                return Err(api::Error::new(self.summary_if_failure.clone(), error));
            }
        };
        let exit_code = match run_outcome {
            Outcome::Completed(exit_code) => exit_code,
            Outcome::Timeout => {
                error!("{} for `{}` timed out", run_spec.command_spec, self.session);
                return Err(api::Error::new(
                    self.summary_if_failure.clone(),
                    anyhow!("Timeout"),
                ));
            }
            Outcome::Cancel => {
                error!("{} for `{}` cancelled", run_spec.command_spec, self.session);
                return Err(api::Error::new(
                    self.summary_if_failure.clone(),
                    anyhow!("Cancelled"),
                ));
            }
        };
        if exit_code == 0 {
            debug!(
                "{} for `{}` successful",
                run_spec.command_spec, self.session
            );
            Ok(())
        } else {
            error!(
                "{} for `{}` exited non-successfully",
                run_spec.command_spec, self.session
            );
            Err(api::Error::new(
                self.summary_if_failure.clone(),
                anyhow!(
                    "Non-zero exit code, see {} for stdio logs",
                    run_spec.runtime_base_path
                ),
            ))
        }
    }
}

struct StepDisableSharedHolotree {
    binary_path: Utf8PathBuf,
    robocorp_home_base: Utf8PathBuf,
    working_directory: Utf8PathBuf,
    session: Session,
    cancellation_token: CancellationToken,
}

impl StepDisableSharedHolotree {
    fn new_from_config(config: &GlobalConfig, session: Session) -> Self {
        Self {
            binary_path: config.rcc_config.binary_path.clone(),
            robocorp_home_base: config.rcc_config.robocorp_home_base.clone(),
            working_directory: config.working_directory_rcc_setup_steps.clone(),
            session,
            cancellation_token: config.cancellation_token.clone(),
        }
    }
}

impl SetupStep for StepDisableSharedHolotree {
    fn setup(&self) -> Result<(), api::Error> {
        let mut command_spec = RCCEnvironment::bundled_command_spec(
            &self.binary_path,
            self.session
                .robocorp_home(&self.robocorp_home_base)
                .to_string(),
        );
        command_spec.add_arguments(["holotree", "init", "--revoke"]);
        debug!("Running {} for `{}`", command_spec, self.session);
        let name = "holotree_disabling_sharing";
        let run_spec = &RunSpec {
            id: &format!("robotmk_{name}_{}", self.session.id()),
            command_spec: &command_spec,
            runtime_base_path: &rcc_working_directory_for_session(
                &self.working_directory,
                &self.session,
            )
            .join(name),
            timeout: 120,
            cancellation_token: &self.cancellation_token,
        };
        match self.session.run(run_spec) {
            Ok(Outcome::Completed(0)) => {
                debug!(
                    "{} for `{}` successful",
                    run_spec.command_spec, self.session
                );
                Ok(())
            }
            Ok(Outcome::Completed(5)) => {
                debug!(
                    "`{}` not using shared holotree. Don't need to disable.",
                    self.session
                );
                Ok(())
            }
            Ok(Outcome::Completed(_)) => Err(api::Error::new(
                "Disabling RCC shared holotree exited non-successfully".into(),
                anyhow!(
                    "Disabling RCC shared holotree exited non-successfully, see {} for stdio logs.",
                    run_spec.runtime_base_path
                ),
            )),
            Ok(Outcome::Timeout) => Err(api::Error::new(
                "Disabling shared holotree timed out".into(),
                anyhow!("Timeout"),
            )),
            Ok(Outcome::Cancel) => {
                error!("{} for `{}` cancelled", run_spec.command_spec, self.session);
                Err(api::Error::new(
                    "Disabling shared holotree cancelled".into(),
                    anyhow!("Cancelled"),
                ))
            }
            Err(error) => {
                let error = error.context(format!(
                    "Failed to run {} for `{}`",
                    run_spec.command_spec, self.session
                ));
                Err(api::Error::new(
                    "Disabling shared holotree failed".into(),
                    error,
                ))
            }
        }
    }
}

#[cfg(windows)]
pub fn gather_rcc_binary_permissions(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepFilePermissions {
                target: config.rcc_config.binary_path.clone(),
                session,
                icacls_permissions: "(RX)".to_string(),
            }),
            plans_in_session,
        ));
    }
    steps
}

#[cfg(windows)]
pub fn gather_rcc_profile_permissions(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    match &config.rcc_config.profile_config {
        RCCProfileConfig::Default => steps.push(skip(rcc_plans)),
        RCCProfileConfig::Custom(custom_profile) => {
            for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
                steps.push((
                    Box::new(StepFilePermissions {
                        target: custom_profile.path.clone(),
                        session,
                        icacls_permissions: "(R)".to_string(),
                    }),
                    plans_in_session,
                ));
            }
        }
    }
    steps
}

pub fn gather_disable_rcc_telemetry(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepRCCCommand::new_from_config(
                config,
                session,
                &["configure", "identity", "--do-not-track"],
                "telemetry_disabling",
                "Disabling RCC telemetry failed",
            )),
            plans_in_session,
        ));
    }
    steps
}

pub fn gather_configure_default_rcc_profile(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    if !matches!(config.rcc_config.profile_config, RCCProfileConfig::Default) {
        return vec![skip(plans)];
    }
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepRCCCommand::new_from_config(
                config,
                session,
                &["configuration", "switch", "--noprofile"],
                "default_profile_switch",
                "Switching to default RCC profile failed",
            )),
            plans_in_session,
        ));
    }
    steps
}

pub fn gather_import_custom_rcc_profile(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let custom_rcc_profile_path = match &config.rcc_config.profile_config {
        RCCProfileConfig::Default => return vec![skip(plans)],
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            custom_rcc_profile_config.path.clone()
        }
    };
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepRCCCommand::new_from_config(
                config,
                session.clone(),
                &[
                    "configuration",
                    "import",
                    "--filename",
                    custom_rcc_profile_path.as_str(),
                ],
                "custom_profile_import",
                "Importing custom RCC profile failed",
            )),
            plans_in_session,
        ));
    }
    steps
}

pub fn gather_switch_to_custom_rcc_profile(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let custom_rcc_profile_name = match &config.rcc_config.profile_config {
        RCCProfileConfig::Default => return vec![skip(plans)],
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            custom_rcc_profile_config.name.clone()
        }
    };
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepRCCCommand::new_from_config(
                config,
                session,
                &[
                    "configuration",
                    "switch",
                    "--profile",
                    custom_rcc_profile_name.as_str(),
                ],
                "custom_profile_switch",
                "Switching to custom RCC porfile failed",
            )),
            plans_in_session,
        ));
    }
    steps
}

#[cfg(windows)]
pub fn gather_enable_rcc_long_path_support(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    vec![
        (
            Box::new(StepRCCCommand::new_from_config(
                config,
                Session::Current(CurrentSession {}),
                &["configure", "longpaths", "--enable"],
                "long_path_support_enabling",
                "Enabling RCC long path support failed",
            )),
            rcc_plans,
        ),
        skip(system_plans),
    ]
}

pub fn gather_disable_rcc_shared_holotree(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_rcc_and_system_plans(plans);
    let mut steps: Vec<StepWithPlans> = vec![skip(system_plans)];
    for (session, plans_in_session) in plans_by_sessions(rcc_plans) {
        steps.push((
            Box::new(StepDisableSharedHolotree::new_from_config(
                global_config,
                session.clone(),
            )),
            plans_in_session,
        ));
    }
    steps
}
