use super::plans_by_sessions;
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan};
use crate::logging::log_and_return_error;
use robotmk::command_spec::CommandSpec;
use robotmk::environment::{Environment, RCCEnvironment};
use robotmk::results::SetupFailure;
use robotmk::session::{RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use robotmk::config::{CustomRCCProfileConfig, RCCProfileConfig};
use std::vec;

pub fn setup(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> AnyhowResult<(Vec<Plan>, Vec<SetupFailure>)> {
    let (rcc_plans, mut system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));

    if rcc_plans.is_empty() {
        sort_plans_by_grouping(&mut system_plans);
        return Ok((system_plans, vec![]));
    }

    #[cfg(windows)]
    let (surviving_rcc_plans, rcc_file_permissions_failures) = {
        use super::windows_permissions::adjust_rcc_file_permissions;
        adjust_rcc_file_permissions(&global_config.rcc_config, rcc_plans)
    };
    #[cfg(unix)]
    let (surviving_rcc_plans, rcc_file_permissions_failures) = (rcc_plans, vec![]);

    let (surviving_rcc_plans, further_rcc_setup_failures) =
        rcc_setup(global_config, surviving_rcc_plans)?;

    let mut surviving_plans = vec![];
    surviving_plans.extend(surviving_rcc_plans);
    surviving_plans.extend(system_plans);
    sort_plans_by_grouping(&mut surviving_plans);
    Ok((
        surviving_plans,
        rcc_file_permissions_failures
            .into_iter()
            .chain(further_rcc_setup_failures)
            .collect(),
    ))
}

pub fn rcc_setup_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("rcc_setup")
}

fn rcc_setup(
    global_config: &GlobalConfig,
    rcc_plans: Vec<Plan>,
) -> AnyhowResult<(Vec<Plan>, Vec<SetupFailure>)> {
    let mut sucessful_plans: Vec<Plan>;
    let mut all_failures = vec![];
    let mut current_failures: Vec<SetupFailure>;

    debug!("Disabling RCC telemetry");
    (sucessful_plans, current_failures) = disable_rcc_telemetry(global_config, rcc_plans)
        .context("Received termination signal while disabling RCC telemetry")?;
    all_failures.extend(current_failures);

    debug!("Configuring RCC profile");
    (sucessful_plans, current_failures) = configure_rcc_profile(global_config, sucessful_plans)
        .context("Received termination signal while configuring RCC profile")?;
    all_failures.extend(current_failures);

    #[cfg(windows)]
    {
        debug!("Enabling support for long paths");
        (sucessful_plans, current_failures) =
            enable_long_path_support(global_config, sucessful_plans)
                .context("Received termination signal while enabling support for long paths")?;
        all_failures.extend(current_failures);
    }

    debug!("Disabling shared holotree");
    (sucessful_plans, current_failures) = holotree_disable_sharing(global_config, sucessful_plans)
        .context("Received termination signal while revoking shared holotree")?;
    all_failures.extend(current_failures);

    Ok((sucessful_plans, all_failures))
}

fn disable_rcc_telemetry(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["configure", "identity", "--do-not-track"]);
    run_command_spec_per_session(
        global_config,
        plans,
        &command_spec,
        "telemetry_disabling",
        "Disabling RCC telemetry failed",
    )
}

fn configure_rcc_profile(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    match &global_config.rcc_config.profile_config {
        RCCProfileConfig::Default => configure_default_rcc_profile(global_config, plans),
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            configure_custom_rcc_profile(custom_rcc_profile_config, global_config, plans)
        }
    }
}

fn configure_default_rcc_profile(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["configuration", "switch", "--noprofile"]);
    run_command_spec_per_session(
        global_config,
        plans,
        &command_spec,
        "default_profile_switch",
        "Switching to default RCC profile failed",
    )
}

fn configure_custom_rcc_profile(
    custom_rcc_profile_config: &CustomRCCProfileConfig,
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut command_spec_import =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec_import.add_arguments([
        "configuration",
        "import",
        "--filename",
        custom_rcc_profile_config.path.as_str(),
    ]);
    let (sucessful_plans_import, failures_import) = run_command_spec_per_session(
        global_config,
        plans,
        &command_spec_import,
        "custom_profile_import",
        "Importing custom RCC profile failed",
    )?;
    let mut command_spec_switch =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec_switch.add_arguments([
        "configuration",
        "switch",
        "--profile",
        custom_rcc_profile_config.name.as_str(),
    ]);
    let (sucessful_plans_switch, failures_switch) = run_command_spec_per_session(
        global_config,
        sucessful_plans_import,
        &command_spec_switch,
        "custom_profile_switch",
        "Switching to custom RCC porfile failed",
    )?;

    Ok((
        sucessful_plans_switch,
        failures_import.into_iter().chain(failures_switch).collect(),
    ))
}

#[cfg(windows)]
fn enable_long_path_support(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
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
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["holotree", "init", "--revoke"]);
    let mut succesful_plans = vec![];
    let mut failures = vec![];

    for (session, plans) in plans_by_sessions(plans) {
        debug!("Running {} for `{}`", command_spec, &session);
        let name = "holotree_disabling_sharing";
        let run_spec = &RunSpec {
            id: &format!("robotmk_{name}_{}", session.id()),
            command_spec: &command_spec,
            base_path: &rcc_setup_working_directory(&global_config.working_directory)
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
                        plan.id, run_spec.base_path
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Disabling RCC shared holotree exited non-successfully"
                            .to_string(),
                        details: format!("See {} for stdio logs", run_spec.base_path),
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
                         Error: {error:#}",
                        plan.id,
                    );
                    failures.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: "Disabling RCC shared holotree failed".to_string(),
                        details: format!("{error:#}"),
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
    let base_path = &rcc_setup_working_directory(&global_config.working_directory)
        .join(session.id())
        .join(id);
    Ok(
        match execute_run_spec_in_session(
            &session,
            &RunSpec {
                id: &format!("robotmk_{id}"),
                command_spec,
                base_path,
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

fn run_command_spec_per_session(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
    command_spec: &CommandSpec,
    id: &str,
    failure_summary: &str,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut succesful_plans = vec![];
    let mut failures = vec![];

    for (session, plans) in plans_by_sessions(plans) {
        let base_path = &rcc_setup_working_directory(&global_config.working_directory)
            .join(session.id())
            .join(id);
        debug!("Running {} for `{}`", command_spec, &session);
        match execute_run_spec_in_session(
            &session,
            &RunSpec {
                id: &format!("robotmk_{id}"),
                command_spec,
                base_path,
                timeout: 120,
                cancellation_token: &global_config.cancellation_token,
            },
        )? {
            Some(error_msg) => {
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
            }
            None => succesful_plans.extend(plans),
        }
    }

    Ok((succesful_plans, failures))
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
            run_spec.base_path
        )))
    }
}
