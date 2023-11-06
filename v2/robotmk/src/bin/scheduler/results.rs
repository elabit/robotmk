use super::internal_config::Suite;
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::section::{WritePiggybackSection, WriteSection};
use serde::Serialize;
use std::collections::HashMap;

pub fn suite_results_directory(results_directory: &Utf8Path) -> Utf8PathBuf {
    results_directory.join("suites")
}

#[derive(Serialize)]
pub struct RCCSetupFailures {
    pub telemetry_disabling: Vec<String>,
    pub shared_holotree: Vec<String>,
    pub holotree_init: Vec<String>,
}

impl WriteSection for RCCSetupFailures {
    fn name() -> &'static str {
        "rcc_setup_failures"
    }
}

pub struct EnvironmentBuildStatesAdministrator {
    build_states: HashMap<String, EnvironmentBuildStatus>,
    path: Utf8PathBuf,
}

#[derive(Serialize)]
pub struct BuildStates<'a>(&'a HashMap<String, EnvironmentBuildStatus>);

impl WriteSection for BuildStates<'_> {
    fn name() -> &'static str {
        "environment_build_states"
    }
}

impl EnvironmentBuildStatesAdministrator {
    pub fn new_with_pending(
        suites: &[Suite],
        results_directory: &Utf8Path,
    ) -> Result<EnvironmentBuildStatesAdministrator> {
        let build_states: HashMap<_, _> = suites
            .iter()
            .map(|suite| (suite.name.to_string(), EnvironmentBuildStatus::Pending))
            .collect();
        let path = results_directory.join("environment_build_states.json");
        BuildStates(&build_states).write(&path)?;
        Ok(Self { build_states, path })
    }

    pub fn update(&mut self, suite_name: &str, build_status: EnvironmentBuildStatus) -> Result<()> {
        self.build_states.insert(suite_name.into(), build_status);
        BuildStates(&self.build_states).write(&self.path)
    }
}

#[derive(Serialize)]
pub enum EnvironmentBuildStatus {
    Success(i64),
    Failure(EnvironmentBuildStatusError),
    NotNeeded,
    Pending,
    InProgress(i64),
}

#[derive(Serialize)]
pub enum EnvironmentBuildStatusError {
    NonZeroExit,
    Timeout,
    Error(String),
}

#[derive(Serialize)]
pub struct SuiteExecutionReport {
    pub suite_name: String,
    pub outcome: ExecutionReport,
}

impl WritePiggybackSection for SuiteExecutionReport {
    fn name() -> &'static str {
        "suite_execution_report"
    }
}

#[derive(Serialize)]
pub enum ExecutionReport {
    Executed(AttemptsOutcome),
    AlreadyRunning,
}

#[derive(Serialize)]
pub struct AttemptsOutcome {
    pub attempts: Vec<AttemptOutcome>,
    pub rebot: Option<RebotOutcome>,
}

#[derive(Serialize)]
pub enum AttemptOutcome {
    AllTestsPassed,
    TestFailures,
    RobotFrameworkFailure,
    EnvironmentFailure,
    TimedOut,
    OtherError(String),
}

#[derive(Serialize)]
pub enum RebotOutcome {
    Ok(RebotResult),
    Error(String),
}

#[derive(Serialize)]
pub struct RebotResult {
    pub xml: String,
    pub html_base64: String,
}
