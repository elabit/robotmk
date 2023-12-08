use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::lock::Locker;
use robotmk::results::{BuildStates, EnvironmentBuildStatus};
use robotmk::section::WriteSection;
use std::collections::HashMap;

pub struct EnvironmentBuildStatesAdministrator<'a> {
    build_states: HashMap<String, EnvironmentBuildStatus>,
    path: Utf8PathBuf,
    locker: &'a Locker,
}

impl<'a> EnvironmentBuildStatesAdministrator<'a> {
    pub fn new_with_pending<'b>(
        ids: impl Iterator<Item = &'b str>,
        results_directory: &Utf8Path,
        locker: &'a Locker,
    ) -> Result<EnvironmentBuildStatesAdministrator<'a>> {
        let build_states: HashMap<_, _> = ids
            .map(|id| (id.to_string(), EnvironmentBuildStatus::Pending))
            .collect();
        let path = results_directory.join("environment_build_states.json");
        BuildStates(&build_states).write(&path, locker)?;
        Ok(Self {
            build_states,
            path,
            locker,
        })
    }

    pub fn update(&mut self, suite_id: &str, build_status: EnvironmentBuildStatus) -> Result<()> {
        self.build_states.insert(suite_id.into(), build_status);
        BuildStates(&self.build_states).write(&self.path, self.locker)
    }
}
