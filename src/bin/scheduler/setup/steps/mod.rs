mod api;
mod conda;
mod directories;
mod long_path_support;
mod rcc;
pub mod run;
mod unpack_managed;

use crate::internal_config::Plan;

use camino::{Utf8Path, Utf8PathBuf};
use robotmk::env::Environment;
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

fn partition_into_rcc_and_other_plans(plans: Vec<Plan>) -> (Vec<Plan>, Vec<Plan>) {
    plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Rcc(_)))
}

fn partition_into_conda_and_other_plans(plans: Vec<Plan>) -> (Vec<Plan>, Vec<Plan>) {
    plans
        .into_iter()
        .partition(|plan| matches!(plan.environment, Environment::Conda(_)))
}

fn rcc_working_directory_for_session(
    working_directory_rcc_setup_steps: &Utf8Path,
    session: &Session,
) -> Utf8PathBuf {
    working_directory_rcc_setup_steps.join(session.id())
}
