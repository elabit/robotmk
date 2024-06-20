pub mod general;
mod icacls;
pub mod rcc;
pub mod zip;
use log::error;

use crate::internal_config::Plan;
use anyhow::Context;
use camino::Utf8Path;
use icacls::run_icacls_command;
use robotmk::session::Session;
use std::collections::HashMap;

fn plans_by_sessions(plans: Vec<Plan>) -> HashMap<Session, Vec<Plan>> {
    let mut plans_by_session = HashMap::new();
    for plan in plans {
        plans_by_session
            .entry(plan.session.clone())
            .or_insert(vec![])
            .push(plan);
    }
    plans_by_session
}

fn grant_permissions_to_all_plan_users(
    path: impl AsRef<Utf8Path>,
    plans: Vec<Plan>,
    permissions: &str,
    additional_icacls_args: &[&str],
) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = vec![];
    let mut failures_by_plan_id = HashMap::new();

    for (session, plans_in_session) in plans_by_sessions(plans) {
        if let Session::User(user_session) = session {
            let icacls_permission_arg = format!("{}:{}", user_session.user_name, permissions);
            let mut icacls_args = vec![path.as_ref().as_str(), "/grant", &icacls_permission_arg];
            icacls_args.extend(additional_icacls_args);

            match run_icacls_command(icacls_args).context(format!(
                "Adjusting permissions of {} for user `{}` failed",
                path.as_ref(),
                user_session.user_name
            )) {
                Ok(_) => surviving_plans.extend(plans_in_session),
                Err(error) => {
                    error!("{error:?}");
                    for plan in plans_in_session {
                        failures_by_plan_id.insert(plan.id, format!("{error:?}"));
                    }
                }
            }
        } else {
            surviving_plans.extend(plans_in_session);
        }
    }

    (surviving_plans, failures_by_plan_id)
}

fn failed_plan_ids_human_readable<'a>(failed_plan_ids: impl Iterator<Item = &'a String>) -> String {
    failed_plan_ids
        .map(|plan_id| plan_id.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}
