use anyhow::{Context, Result};
use atomicwrites::{AtomicFile, OverwriteBehavior};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use serde_json::to_string;
use std::{collections::HashMap, io::Write};

pub fn suite_results_directory(results_directory: &Utf8Path) -> Utf8PathBuf {
    results_directory.join("suites")
}

pub fn write_file_atomic(
    content: &str,
    working_directory: &Utf8Path,
    final_path: &Utf8PathBuf,
) -> Result<()> {
    AtomicFile::new_with_tmpdir(
        final_path,
        OverwriteBehavior::AllowOverwrite,
        working_directory,
    )
    .write(|f| f.write_all(content.as_bytes()))
    .context(format!(
        "Atomic write failed. Working directory: {working_directory}, final path: {final_path}.",
    ))
}

pub struct EnvironmentBuildStatesAdministrator<'a> {
    build_states: HashMap<&'a String, EnvironmentBuildStatus>,
    working_directory: &'a Utf8Path,
    results_directory: &'a Utf8Path,
}

impl<'a> EnvironmentBuildStatesAdministrator<'a> {
    pub fn new_with_pending(
        suite_names: impl Iterator<Item = &'a String>,
        working_directory: &'a Utf8Path,
        results_directory: &'a Utf8Path,
    ) -> EnvironmentBuildStatesAdministrator<'a> {
        Self {
            build_states: HashMap::from_iter(
                suite_names.map(|suite_name| (suite_name, EnvironmentBuildStatus::Pending)),
            ),
            working_directory,
            results_directory,
        }
    }

    pub fn write_atomic(&self) -> Result<()> {
        write_file_atomic(
            &to_string(&self.build_states)
                .context("Serializing environment build states failed")?,
            self.working_directory,
            &self.results_directory.join("environment_build_states.json"),
        )
        .context("Writing environment build states failed")
    }

    pub fn insert_and_write_atomic(
        &mut self,
        suite_name: &'a String,
        environment_build_status: EnvironmentBuildStatus,
    ) -> Result<()> {
        self.build_states
            .insert(suite_name, environment_build_status);
        self.write_atomic()
    }
}

#[derive(Serialize)]
pub enum EnvironmentBuildStatus {
    Success,
    Failure(EnvironmentBuildStatusError),
    NotNeeded,
    Pending,
    InProgress,
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
