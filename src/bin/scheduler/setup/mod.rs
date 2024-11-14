pub mod api;
pub mod directories;
pub mod rcc;
pub mod unpack_managed;
pub mod windows_permissions;

use crate::internal_config::Plan;
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
