#![cfg(windows)]
use super::plans_by_sessions;
use anyhow::{bail, Context};
use log::{debug, error};
use std::process::Command;

use crate::internal_config::Plan;
use camino::Utf8Path;
use robotmk::config::{RCCConfig, RCCProfileConfig};
use robotmk::results::SetupFailure;
use robotmk::session::Session;

pub fn run_icacls_command<'a>(arguments: impl IntoIterator<Item = &'a str>) -> anyhow::Result<()> {
    let mut command = Command::new("icacls.exe");
    command.args(arguments);
    let output = command
        .output()
        .context(format!("Calling icacls.exe failed. Command:\n{command:?}"))?;
    if !output.status.success() {
        bail!(
            "icacls.exe exited non-successfully.\n\nCommand:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    Ok(())
}

pub fn grant_permissions_to_all_plan_users(
    path: &Utf8Path,
    plans: Vec<Plan>,
    permissions: &str,
    description_for_failure_reporting: &str,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = vec![];
    let mut failures = vec![];

    for (session, plans_in_session) in plans_by_sessions(plans) {
        if let Session::User(user_session) = session {
            match run_icacls_command([
                path.as_str(),
                "/grant",
                &format!("{}:{}", user_session.user_name, permissions),
                "/T",
                "/L",
            ])
            .context(format!(
                "Adjusting permissions of {path} for user `{}` failed",
                user_session.user_name
            )) {
                Ok(_) => surviving_plans.extend(plans_in_session),
                Err(error) => {
                    for plan in plans_in_session {
                        error!(
                            "Plan {}: Failed to adjust permissions of \
                             {description_for_failure_reporting} for plan user. Plan won't be scheduled.
                             Error: {error:?}",
                            plan.id
                        );
                        failures.push(SetupFailure {
                            plan_id: plan.id.clone(),
                            summary: format!(
                                "Failed to adjust permissions of {description_for_failure_reporting} for plan user"
                            ),
                            details: format!("{error:?}"),
                        });
                    }
                }
            }
        } else {
            surviving_plans.extend(plans_in_session);
        }
    }

    (surviving_plans, failures)
}

pub fn grant_full_access(user: &str, target_path: &Utf8Path) -> anyhow::Result<()> {
    let arguments = [
        target_path.as_ref(),
        "/grant",
        &format!("{user}:(OI)(CI)F"),
        "/T",
        "/L",
    ];
    run_icacls_command(arguments).map_err(|e| {
        let message = format!("Adjusting permissions of {target_path} for user `{user}` failed");
        e.context(message)
    })
}

pub fn reset_access(target_path: &Utf8Path) -> anyhow::Result<()> {
    let arguments = [target_path.as_ref(), "/reset", "/T", "/L"];
    run_icacls_command(arguments).map_err(|e| {
        let message = format!("Resetting permissions of {target_path} failed");
        e.context(message)
    })
}

pub fn adjust_rcc_file_permissions(
    rcc_config: &RCCConfig,
    rcc_plans: Vec<Plan>,
) -> (Vec<Plan>, Vec<SetupFailure>) {
    debug!(
        "Granting all plan users read and execute access to {}",
        rcc_config.binary_path
    );
    let (mut surviving_rcc_plans, rcc_binary_permissions_failures) =
        grant_permissions_to_all_plan_users(
            &rcc_config.binary_path,
            rcc_plans,
            "(RX)",
            "RCC binary",
        );

    let mut rcc_profile_file_permissions_failures = vec![];
    if let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config {
        debug!(
            "Granting all plan users read access to {}",
            custom_rcc_profile_config.path
        );
        (surviving_rcc_plans, rcc_profile_file_permissions_failures) =
            grant_permissions_to_all_plan_users(
                &custom_rcc_profile_config.path,
                surviving_rcc_plans,
                "(R)",
                "RCC profile file",
            );
    }

    (
        surviving_rcc_plans,
        rcc_binary_permissions_failures
            .into_iter()
            .chain(rcc_profile_file_permissions_failures)
            .collect(),
    )
}
