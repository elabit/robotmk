use super::windows_permissions::grant_permissions_to_all_plan_users;
use super::{failed_plan_ids_human_readable, plans_by_sessions};
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan};
use crate::logging::log_and_return_error;
use robotmk::command_spec::CommandSpec;
use robotmk::environment::{Environment, RCCEnvironment};
use robotmk::results::RCCSetupFailures;
use robotmk::session::{CurrentSession, RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use robotmk::{
    config::{CustomRCCProfileConfig, RCCConfig, RCCProfileConfig},
    section::WriteSection,
};
use std::collections::HashMap;
use std::vec;

pub fn setup(global_config: &GlobalConfig, plans: Vec<Plan>) -> AnyhowResult<Vec<Plan>> {
    let (rcc_plans, mut system_plans): (Vec<Plan>, Vec<Plan>) = plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)));

    if rcc_plans.is_empty() {
        sort_plans_by_grouping(&mut system_plans);
        return Ok(system_plans);
    }

    let mut rcc_setup_failures = RCCSetupFailures::default();
    let surviving_rcc_plans = adjust_rcc_file_permissions(
        &global_config.rcc_config,
        rcc_plans,
        &mut rcc_setup_failures,
    );
    let surviving_rcc_plans =
        rcc_setup(global_config, surviving_rcc_plans, &mut rcc_setup_failures)?;

    rcc_setup_failures.write(
        global_config
            .results_directory
            .join("rcc_setup_failures.json"),
        &global_config.results_directory_locker,
    )?;

    let mut surviving_plans = vec![];
    surviving_plans.extend(surviving_rcc_plans);
    surviving_plans.extend(system_plans);
    sort_plans_by_grouping(&mut surviving_plans);
    Ok(surviving_plans)
}

pub fn rcc_setup_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("rcc_setup")
}

fn adjust_rcc_file_permissions(
    rcc_config: &RCCConfig,
    rcc_plans: Vec<Plan>,
    rcc_setup_failures: &mut RCCSetupFailures,
) -> Vec<Plan> {
    let mut surviving_rcc_plans: Vec<Plan>;

    debug!(
        "Granting all plan users read and execute access to {}",
        rcc_config.binary_path
    );
    (surviving_rcc_plans, rcc_setup_failures.binary_permissions) =
        grant_permissions_to_all_plan_users(&rcc_config.binary_path, rcc_plans, "(RX)", &[]);
    if !rcc_setup_failures.binary_permissions.is_empty() {
        error!(
            "Dropping the following plans due to failure to adjust RCC binary permissions: {}",
            failed_plan_ids_human_readable(rcc_setup_failures.binary_permissions.keys())
        );
    }

    if let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config {
        debug!(
            "Granting all plan users read access to {}",
            custom_rcc_profile_config.path
        );
        (surviving_rcc_plans, rcc_setup_failures.profile_permissions) =
            grant_permissions_to_all_plan_users(
                &custom_rcc_profile_config.path,
                surviving_rcc_plans,
                "(R)",
                &[],
            );
        if !rcc_setup_failures.profile_permissions.is_empty() {
            error!(
                "Dropping the following plans due to failure to adjust RCC profile permissions: {}",
                failed_plan_ids_human_readable(rcc_setup_failures.profile_permissions.keys())
            );
        }
    }

    surviving_rcc_plans
}

fn rcc_setup(
    global_config: &GlobalConfig,
    rcc_plans: Vec<Plan>,
    rcc_setup_failures: &mut RCCSetupFailures,
) -> AnyhowResult<Vec<Plan>> {
    let mut sucessful_plans: Vec<Plan>;

    debug!("Disabling RCC telemetry");
    (sucessful_plans, rcc_setup_failures.telemetry_disabling) =
        disable_rcc_telemetry(global_config, rcc_plans)
            .context("Received termination signal while disabling RCC telemetry")?;
    if !rcc_setup_failures.telemetry_disabling.is_empty() {
        error!(
            "Dropping the following plans due to RCC telemetry disabling failure: {}",
            failed_plan_ids_human_readable(rcc_setup_failures.telemetry_disabling.keys())
        );
    }

    debug!("Configuring RCC profile");
    (sucessful_plans, rcc_setup_failures.profile_configuring) =
        configure_rcc_profile(global_config, sucessful_plans)
            .context("Received termination signal while configuring RCC profile")?;
    if !rcc_setup_failures.profile_configuring.is_empty() {
        error!(
            "Dropping the following plans due to profile configuring failure: {}",
            failed_plan_ids_human_readable(rcc_setup_failures.profile_configuring.keys())
        );
    }

    debug!("Enabling support for long paths");
    (sucessful_plans, rcc_setup_failures.long_path_support) =
        enable_long_path_support(global_config, sucessful_plans)
            .context("Received termination signal while enabling support for long paths")?;
    if !rcc_setup_failures.long_path_support.is_empty() {
        error!(
            "Dropping the following plans due to long path support enabling failure: {}",
            failed_plan_ids_human_readable(rcc_setup_failures.long_path_support.keys())
        );
    }

    debug!("Disabling shared holotree");
    (
        sucessful_plans,
        rcc_setup_failures.holotree_disabling_sharing,
    ) = holotree_disable_sharing(global_config, sucessful_plans)
        .context("Received termination signal while revoking shared holotree")?;
    if !rcc_setup_failures.holotree_disabling_sharing.is_empty() {
        error!(
            "Dropping the following plans due to failing to disable holotree sharing: {}",
            failed_plan_ids_human_readable(rcc_setup_failures.holotree_disabling_sharing.keys())
        );
    }

    Ok(sucessful_plans)
}

fn disable_rcc_telemetry(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["configure", "identity", "--do-not-track"]);
    run_command_spec_per_session(global_config, plans, &command_spec, "telemetry_disabling")
}

fn configure_rcc_profile(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
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
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["configuration", "switch", "--noprofile"]);
    run_command_spec_per_session(
        global_config,
        plans,
        &command_spec,
        "default_profile_switch",
    )
}

fn configure_custom_rcc_profile(
    custom_rcc_profile_config: &CustomRCCProfileConfig,
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut command_spec_import =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec_import.add_arguments([
        "configuration",
        "import",
        "--filename",
        custom_rcc_profile_config.path.as_str(),
    ]);
    let (sucessful_plans_import, failed_plans_import) = run_command_spec_per_session(
        global_config,
        plans,
        &command_spec_import,
        "custom_profile_import",
    )?;
    let mut command_spec_switch =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec_switch.add_arguments([
        "configuration",
        "switch",
        "--profile",
        custom_rcc_profile_config.name.as_str(),
    ]);
    let (sucessful_plans_switch, failed_plans_switch) = run_command_spec_per_session(
        global_config,
        sucessful_plans_import,
        &command_spec_switch,
        "custom_profile_switch",
    )?;
    let mut failed_plans = HashMap::new();
    failed_plans.extend(failed_plans_import);
    failed_plans.extend(failed_plans_switch);
    Ok((sucessful_plans_switch, failed_plans))
}

fn enable_long_path_support(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["configure", "longpaths", "--enable"]);
    run_command_spec_once_in_current_session(
        global_config,
        plans,
        &command_spec,
        "long_path_support_enabling",
    )
}

fn holotree_disable_sharing(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut command_spec =
        RCCEnvironment::bundled_command_spec(&global_config.rcc_config.binary_path);
    command_spec.add_arguments(["holotree", "init", "--revoke"]);
    let mut succesful_plans = vec![];
    let mut failed_plans: HashMap<String, String> = HashMap::new();

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
                error!(
                    "{} for `{session}` exited non-successfully",
                    run_spec.command_spec
                );
                let error_message = format!(
                    "Non-zero exit code, see {} for stdio logs",
                    run_spec.base_path
                );
                for plan in plans {
                    failed_plans.insert(plan.id, error_message.clone());
                }
            }
            Ok(Outcome::Timeout) => {
                error!("{} for `{session}` timed out", run_spec.command_spec);
                for plan in plans {
                    failed_plans.insert(plan.id, "Timeout".to_string());
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
                let error = log_and_return_error(error);
                let error_message = format!("{error:?}");
                for plan in plans {
                    failed_plans.insert(plan.id, error_message.clone());
                }
            }
        }
    }

    Ok((succesful_plans, failed_plans))
}

fn run_command_spec_once_in_current_session(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
    command_spec: &CommandSpec,
    id: &str,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
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
            None => (plans, HashMap::new()),
            Some(error_msg) => (
                vec![],
                HashMap::from_iter(plans.into_iter().map(|plan| (plan.id, error_msg.clone()))),
            ),
        },
    )
}

fn run_command_spec_per_session(
    global_config: &GlobalConfig,
    plans: Vec<Plan>,
    command_spec: &CommandSpec,
    id: &str,
) -> Result<(Vec<Plan>, HashMap<String, String>), Cancelled> {
    let mut succesful_plans = vec![];
    let mut failed_plans: HashMap<String, String> = HashMap::new();

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
                    failed_plans.insert(plan.id, error_msg.clone());
                }
            }
            None => succesful_plans.extend(plans),
        }
    }

    Ok((succesful_plans, failed_plans))
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
