pub mod general;
mod icacls;
pub mod rcc;

use crate::internal_config::Suite;
use robotmk::session::Session;
use std::collections::HashSet;

fn all_configured_users<'a>(suites: impl Iterator<Item = &'a Suite>) -> Vec<&'a str> {
    let all_users_unique: HashSet<&str> =
        HashSet::from_iter(suites.filter_map(|suite| match &suite.session {
            Session::Current(_) => None,
            Session::User(user_session) => Some(user_session.user_name.as_str()),
        }));
    let mut all_users_sorted: Vec<&str> = all_users_unique.into_iter().collect();
    all_users_sorted.sort();
    all_users_sorted
}
