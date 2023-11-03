use super::internal_config::Suite;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use serde_json::to_string;
use std::{collections::HashMap, io::Write};
use tempfile::NamedTempFile;

pub fn suite_results_directory(results_directory: &Utf8Path) -> Utf8PathBuf {
    results_directory.join("suites")
}

pub fn write_file_atomic(content: &str, final_path: impl AsRef<Utf8Path>) -> Result<()> {
    let mut file = NamedTempFile::new().context("Opening tempfile failed")?;
    file.write_all(content.as_bytes()).context(format!(
        "Writing tempfile failed, {}",
        file.path().display()
    ))?;
    file.persist(final_path.as_ref())
        .context(format!(
            "Persisting tempfile failed, final_path: {}",
            final_path.as_ref()
        ))
        .map(|_| ())
}

#[derive(Serialize)]
pub struct RCCSetupFailures {
    pub telemetry_disabling: Vec<String>,
    pub shared_holotree: Vec<String>,
    pub holotree_init: Vec<String>,
}

impl RCCSetupFailures {
    pub fn write_atomic(&self, results_directory: &Utf8Path) -> Result<()> {
        write_file_atomic(
            &to_string(&self)?,
            results_directory.join("rcc_setup_failures.json"),
        )
        .context("Writing RCC setup failures failed")
    }
}

pub struct EnvironmentBuildStatesAdministrator<'a> {
    build_states: HashMap<String, EnvironmentBuildStatus>,
    results_directory: &'a Utf8Path,
}

impl<'a> EnvironmentBuildStatesAdministrator<'a> {
    pub fn new_with_pending(
        suites: &[Suite],
        results_directory: &'a Utf8Path,
    ) -> EnvironmentBuildStatesAdministrator<'a> {
        Self {
            build_states: HashMap::from_iter(
                suites
                    .iter()
                    .map(|suite| (suite.name.to_string(), EnvironmentBuildStatus::Pending)),
            ),
            results_directory,
        }
    }

    pub fn write_atomic(&self) -> Result<()> {
        write_file_atomic(
            &to_string(&self.build_states)
                .context("Serializing environment build states failed")?,
            self.results_directory.join("environment_build_states.json"),
        )
        .context("Writing environment build states failed")
    }

    pub fn insert_and_write_atomic(
        &mut self,
        suite_name: &str,
        environment_build_status: EnvironmentBuildStatus,
    ) -> Result<()> {
        self.build_states
            .insert(suite_name.to_string(), environment_build_status);
        self.write_atomic()
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
