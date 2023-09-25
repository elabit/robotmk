use anyhow::{Context, Result};
use atomicwrites::{AtomicFile, OverwriteBehavior};
use serde::Serialize;
use serde_json::to_string;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, io::Write};

pub fn suite_results_directory(results_directory: &Path) -> PathBuf {
    results_directory.join("suites")
}

pub fn suite_result_file(suite_results_dir: &Path, suite_name: &str) -> PathBuf {
    suite_results_dir.join(format!("{}.json", suite_name))
}

pub fn write_file_atomic(content: &str, working_directory: &Path, final_path: &Path) -> Result<()> {
    AtomicFile::new_with_tmpdir(
        final_path,
        OverwriteBehavior::AllowOverwrite,
        working_directory,
    )
    .write(|f| f.write_all(content.as_bytes()))
    .context(format!(
        "Atomic write failed. Working directory: {}, final path: {}.",
        working_directory.display(),
        final_path.display()
    ))
}

pub struct EnvironmentBuildStatesAdministrator<'a> {
    build_states: HashMap<&'a String, EnvironmentBuildStatus>,
    working_directory: &'a Path,
    results_directory: &'a Path,
}

impl<'a> EnvironmentBuildStatesAdministrator<'a> {
    pub fn new_with_pending(
        suite_names: impl Iterator<Item = &'a String>,
        working_directory: &'a Path,
        results_directory: &'a Path,
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
    Failure,
    Timeout,
    NotNeeded,
    Pending,
    InProgress,
}

#[derive(Serialize)]
pub struct SuiteExecutionReport {
    pub suite_name: String,
    pub outcome: ExecutionReport,
}

#[derive(Serialize)]
pub enum ExecutionReport {
    Executed(AttemptsOutcome),
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
