use super::api::{self, run_steps, skip, SetupStep, StepWithPlans};
use super::plans_by_sessions;
#[cfg(windows)]
use super::windows_permissions::run_icacls_command;
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan};
use crate::logging::log_and_return_error;
#[cfg(windows)]
use robotmk::command_spec::CommandSpec;
use robotmk::environment::{Environment, RCCEnvironment};
use robotmk::results::SetupFailure;
use robotmk::session::{RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use robotmk::config::RCCProfileConfig;
use std::vec;
use tokio_util::sync::CancellationToken;

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let (surviving_plans, failures) = run_setup_steps(global_config, plans);

    let (rcc_plans, mut system_plans): (Vec<Plan>, Vec<Plan>) = surviving_plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));

    if rcc_plans.is_empty() {
        sort_plans_by_grouping(&mut system_plans);
        return Ok((system_plans, vec![]));
    }

    let (surviving_rcc_plans, further_rcc_setup_failures) = rcc_setup(global_config, rcc_plans)?;

    let mut surviving_plans = vec![];
    surviving_plans.extend(surviving_rcc_plans);
    surviving_plans.extend(system_plans);
    sort_plans_by_grouping(&mut surviving_plans);
    Ok((
        surviving_plans,
        failures
            .into_iter()
            .chain(further_rcc_setup_failures)
            .collect(),
    ))
}

pub fn rcc_setup_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("rcc_setup")
}

fn run_setup_steps(config: &GlobalConfig, mut plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let gather_requirements = [
        #[cfg(windows)]
        gather_rcc_binary_permissions,
        #[cfg(windows)]
        gather_rcc_profile_permissions,
        gather_disable_rcc_telemetry,
        gather_configure_default_rcc_profile,
        gather_import_custom_rcc_profile,
        gather_switch_to_custom_rcc_profile,
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
            working_directory: config.working_directory.clone(),
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
        debug!("Running {} for `{}`", command_spec, &self.session);
        match execute_run_spec_in_session(
            &self.session,
            &RunSpec {
                id: &format!("robotmk_{}", self.id),
                command_spec: &command_spec,
                runtime_base_path: &rcc_setup_working_directory(&self.working_directory)
                    .join(self.session.id())
                    .join(&self.id),
                timeout: 120,
                cancellation_token: &self.cancellation_token,
            },
        )
        .map_err(|_cancelled| {
            api::Error::new(self.summary_if_failure.clone(), anyhow!("Cancelled"))
        })? {
            Some(error_msg) => Err(api::Error::new(
                self.summary_if_failure.clone(),
                anyhow!(error_msg),
            )),
            None => Ok(()),
        }
    }
}

#[cfg(windows)]
fn gather_rcc_binary_permissions(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();
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
    steps.push(skip(system_plans));
    steps
}

#[cfg(windows)]
fn gather_rcc_profile_permissions(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();

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
    steps.push(skip(system_plans));
    steps
}

fn gather_disable_rcc_telemetry(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();
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
    steps.push(skip(system_plans));
    steps
}

fn gather_configure_default_rcc_profile(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    if !matches!(config.rcc_config.profile_config, RCCProfileConfig::Default) {
        return vec![skip(plans)];
    }
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();
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
    steps.push(skip(system_plans));
    steps
}

fn gather_import_custom_rcc_profile(config: &GlobalConfig, plans: Vec<Plan>) -> Vec<StepWithPlans> {
    let custom_rcc_profile_path = match &config.rcc_config.profile_config {
        RCCProfileConfig::Default => return vec![skip(plans)],
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            custom_rcc_profile_config.path.clone()
        }
    };
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();
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
    steps.push(skip(system_plans));
    steps
}

fn gather_switch_to_custom_rcc_profile(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let custom_rcc_profile_name = match &config.rcc_config.profile_config {
        RCCProfileConfig::Default => return vec![skip(plans)],
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            custom_rcc_profile_config.name.clone()
        }
    };
    let (rcc_plans, system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));
    let mut steps: Vec<StepWithPlans> = Vec::new();
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
    steps.push(skip(system_plans));
    steps
}

fn rcc_setup(
    global_config: &GlobalConfig,
    rcc_plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut all_failures = vec![];

    #[cfg(windows)]
    let successful_plans = {
        debug!("Enabling support for long paths");
        let (successful_plans, current_failures) =
            enable_long_path_support(global_config, rcc_plans)?;
        all_failures.extend(current_failures);
        successful_plans
    };
    #[cfg(unix)]
    let successful_plans = rcc_plans;

    debug!("Disabling shared holotree");
    let (sucessful_plans, current_failures) =
        holotree_disable_sharing(global_config, successful_plans)?;
    all_failures.extend(current_failures);

    Ok((sucessful_plans, all_failures))
}

#[cfg(windows)]
fn enable_long_path_support(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    use robotmk::session::CurrentSession;
    let session = Session::Current(CurrentSession {});
    let mut command_spec = RCCEnvironment::bundled_command_spec(
        &global_config.rcc_config.binary_path,
        session
            .robocorp_home(&global_config.rcc_config.robocorp_home_base)
            .to_string(),
    );
    command_spec.add_arguments(["configure", "longpaths", "--enable"]);
    run_command_spec_once_in_current_session(
        global_config,
        plans,
        &command_spec,
        "long_path_support_enabling",
        "Enabling RCC long path support failed",
    )
}

fn holotree_disable_sharing(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut succesful_plans = vec![];
    let mut failures = vec![];

    for (session, plans) in plans_by_sessions(plans) {
        let mut command_spec = RCCEnvironment::bundled_command_spec(
            &global_config.rcc_config.binary_path,
            session
                .robocorp_home(&global_config.rcc_config.robocorp_home_base)
                .to_string(),
        );
        command_spec.add_arguments(["holotree", "init", "--revoke"]);
        debug!("Running {} for `{}`", command_spec, &session);
        let name = "holotree_disabling_sharing";
        let run_spec = &RunSpec {
            id: &format!("robotmk_{name}_{}", session.id()),
            command_spec: &command_spec,
            runtime_base_path: &rcc_setup_working_directory(&global_config.working_directory)
                .join(session.id())
                .join(name),
            timeout: 120,
            cancellation_token: &global_config.cancellation_token,
        };
        match session.run(run_spec) {
            Ok(Outcome::Completed(0)) => {
                debug!("{} for `{session}` successful", run_spec.command_spec);
                succesful_plans.extend(plans);
            }
            Ok(Outcome::Completed(5)) => {
                debug!("`{session}` not using shared holotree. Don't need to disable.");
                succesful_plans.extend(plans);
            }
            Ok(Outcome::Completed(_)) => {
                for plan in plans {
                    error!(
                        "Plan {}: Disabling RCC shared holotree exited non-successfully, see {} \
                         for stdio logs. Plan won't be scheduled.",
                        plan.id, run_spec.runtime_base_path
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Disabling RCC shared holotree exited non-successfully"
                            .to_string(),
                        details: format!("See {} for stdio logs", run_spec.runtime_base_path),
                    });
                }
            }
            Ok(Outcome::Timeout) => {
                for plan in plans {
                    error!(
                        "Plan {}: Disabling RCC shared holotree timed out. Plan won't be scheduled.",
                        plan.id
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Disabling RCC shared holotree timed out".to_string(),
                        details: format!("{} took longer than 120 seconds", run_spec.command_spec),
                    });
                }
            }
            Ok(Outcome::Cancel) => {
                error!("{} for `{session}` cancelled", run_spec.command_spec);
                return Err(Cancelled {});
            }
            Err(error) => {
                let error = error.context(format!(
                    "Failed to run {} for `{session}`",
                    run_spec.command_spec
                ));
                for plan in plans {
                    error!(
                        "Plan {}: Disabling RCC shared holotree failed. Plan won't be scheduled.
                         Error: {error:?}",
                        plan.id,
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Disabling RCC shared holotree failed".to_string(),
                        details: format!("{error:?}"),
                    });
                }
            }
        }
    }

    Ok((succesful_plans, failures))
}

#[cfg(windows)]
fn run_command_spec_once_in_current_session(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
    command_spec: &CommandSpec,
    id: &str,
    failure_summary: &str,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    use robotmk::session::CurrentSession;
    let session = Session::Current(CurrentSession {});
    Ok(
        match execute_run_spec_in_session(
            &session,
            &RunSpec {
                id: &format!("robotmk_{id}"),
                command_spec,
                runtime_base_path: &rcc_setup_working_directory(&global_config.working_directory)
                    .join(session.id())
                    .join(id),
                timeout: 120,
                cancellation_token: &global_config.cancellation_token,
            },
        )? {
            None => (plans, vec![]),
            Some(error_msg) => {
                let mut failures = vec![];
                for plan in plans {
                    error!(
                        "Plan {}: {failure_summary}. Plan won't be scheduled.
                         Error: {error_msg}",
                        plan.id
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: failure_summary.to_string(),
                        details: error_msg.clone(),
                    });
                }
                (vec![], failures)
            }
        },
    )
}

fn execute_run_spec_in_session(
    session: &Session,
    run_spec: &RunSpec,
) -> Result<Option<String>, Cancelled> {
    let run_outcome = match session.run(run_spec).context(format!(
        "Failed to run {} for `{session}`",
        run_spec.command_spec
    )) {
        Ok(run_outcome) => run_outcome,
        Err(error) => {
            let error = log_and_return_error(error);
            return Ok(Some(format!("{error:?}")));
        }
    };
    let exit_code = match run_outcome {
        Outcome::Completed(exit_code) => exit_code,
        Outcome::Timeout => {
            error!("{} for `{session}` timed out", run_spec.command_spec);
            return Ok(Some("Timeout".into()));
        }
        Outcome::Cancel => {
            error!("{} for `{session}` cancelled", run_spec.command_spec);
            return Err(Cancelled {});
        }
    };
    if exit_code == 0 {
        debug!("{} for `{session}` successful", run_spec.command_spec);
        Ok(None)
    } else {
        error!(
            "{} for `{session}` exited non-successfully",
            run_spec.command_spec
        );
        Ok(Some(format!(
            "Non-zero exit code, see {} for stdio logs",
            run_spec.runtime_base_path
        )))
    }
}
