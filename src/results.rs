use crate::config::PlanMetadata;
use crate::section::{WritePiggybackSection, WriteSection};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use std::collections::HashMap;

pub fn plan_results_directory(results_directory: &Utf8Path) -> Utf8PathBuf {
    results_directory.join("plans")
}

#[derive(Serialize)]
pub enum SchedulerPhase {
    GracePeriod(u64),
    RCCSetup,
    EnvironmentBuilding,
    Scheduling,
}

impl WriteSection for SchedulerPhase {
    fn name() -> &'static str {
        "robotmk_scheduler_phase"
    }
}

#[derive(Default, Serialize)]
pub struct GeneralSetupFailures {
    pub working_directory_permissions: HashMap<String, String>,
}

impl WriteSection for GeneralSetupFailures {
    fn name() -> &'static str {
        "robotmk_general_setup_failures"
    }
}

#[derive(Default, Serialize)]
pub struct RCCSetupFailures {
    pub binary_permissions: HashMap<String, String>,
    pub profile_permissions: HashMap<String, String>,
    pub telemetry_disabling: HashMap<String, String>,
    pub profile_configuring: HashMap<String, String>,
    pub long_path_support: HashMap<String, String>,
    pub shared_holotree: HashMap<String, String>,
    pub holotree_init: HashMap<String, String>,
}

impl WriteSection for RCCSetupFailures {
    fn name() -> &'static str {
        "robotmk_rcc_setup_failures_v2"
    }
}

#[derive(Serialize)]
pub struct BuildStates<'a>(pub &'a HashMap<String, EnvironmentBuildStage>);

impl WriteSection for BuildStates<'_> {
    fn name() -> &'static str {
        "robotmk_environment_build_states"
    }
}

#[derive(PartialEq, Debug, Serialize, Clone)]
pub enum BuildOutcome {
    NotNeeded,
    Success(i64),
    Timeout,
    Error(String),
}

#[derive(Serialize)]
pub enum EnvironmentBuildStage {
    Pending,
    InProgress(i64),
    Complete(BuildOutcome),
}

#[derive(Serialize)]
pub struct PlanExecutionReport {
    pub plan_id: String,
    pub timestamp: i64,
    pub attempts: Vec<AttemptReport>,
    pub rebot: Option<RebotOutcome>,
    pub config: AttemptsConfig,
    pub metadata: PlanMetadata,
}

impl WritePiggybackSection for PlanExecutionReport {
    fn name() -> &'static str {
        "robotmk_plan_execution_report"
    }
}

#[derive(PartialEq, Debug, Serialize)]
pub struct AttemptReport {
    pub index: usize,
    pub outcome: AttemptOutcome,
    pub runtime: i64,
}

#[derive(PartialEq, Debug, Serialize)]
pub enum AttemptOutcome {
    AllTestsPassed,
    TestFailures,
    RobotFailure,
    EnvironmentFailure,
    TimedOut,
    OtherError(String),
}

#[derive(Debug, Serialize)]
pub enum RebotOutcome {
    Ok(RebotResult),
    Error(String),
}

#[derive(Debug, Serialize)]
pub struct RebotResult {
    pub xml: String,
    pub html_base64: String,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct AttemptsConfig {
    pub interval: u64,
    pub timeout: u64,
    pub n_attempts_max: usize,
}
