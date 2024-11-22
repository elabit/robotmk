mod api;
mod directories;
mod rcc;
pub mod run;

use crate::internal_config::Plan;

use robotmk::environment::Environment;
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

fn partition_into_rcc_and_system_plans(plans: Vec<Plan>) -> (Vec<Plan>, Vec<Plan>) {
    plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)))
}
