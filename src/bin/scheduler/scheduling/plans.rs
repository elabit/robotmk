use crate::internal_config::{Plan, Source};
use crate::logging::TIMESTAMP_FORMAT;
use robotmk::config::PlanMetadata;
use robotmk::plans::{PlanRunSpec, run_attempts_with_rebot};
use robotmk::results::{AttemptsConfig, PlanExecutionReport};

use anyhow::Context;
use jiff::Timestamp;
use log::info;
use robotmk::section::WritePiggybackSection;
use robotmk::termination::{ContextUnrecoverable, Terminate};
use std::fs::create_dir_all;

pub fn run_plan(plan: &Plan) -> Result<PlanExecutionReport, Terminate> {
    info!(
        "Running plan {} ({})",
        plan.id,
        format_source_for_logging(&plan.source)
    );
    let report = produce_plan_results(plan)?;
    info!("Plan {} finished", plan.id);
    Ok(report)
}

pub fn write_plan_result(plan: &Plan, report: &PlanExecutionReport) -> Result<(), Terminate> {
    report
        .write(
            &plan.results_file,
            plan.host.clone(),
            &plan.results_directory_locker,
        )
        .context_unrecoverable("Reporting plan results failed")?;
    Ok(())
}

fn format_source_for_logging(source: &Source) -> String {
    match source {
        Source::Manual => "manual robot".to_string(),
        Source::Managed {
            version_number,
            version_label,
            ..
        } => {
            format!(
                "managed robot, version: {}{}",
                version_number,
                if version_label.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({version_label})")
                }
            )
        }
    }
}

fn produce_plan_results(plan: &Plan) -> Result<PlanExecutionReport, Terminate> {
    let timestamp = Timestamp::now();
    let run_id = timestamp.strftime(TIMESTAMP_FORMAT).to_string();
    let output_directory = plan.working_directory.join(&run_id);

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for plan run: {output_directory}"
    ))?;

    let (attempt_reports, rebot) = run_attempts_with_rebot(&PlanRunSpec {
        robot: &plan.robot,
        id: &plan.id,
        environment: &plan.environment,
        session: &plan.session,
        timeout: plan.timeout,
        cancellation_token: &plan.cancellation_token,
        output_directory: &output_directory,
        resource_attributes: &resource_attributes(
            &plan.metadata,
            plan.machine_id.as_deref(),
            &run_id,
        ),
    })
    .map_err(|cancelled| cancelled.into())
    .context_unrecoverable("Received termination signal while running plan")?;

    Ok(PlanExecutionReport {
        plan_id: plan.id.clone(),
        timestamp: timestamp.as_second(),
        attempts: attempt_reports,
        rebot,
        config: AttemptsConfig {
            interval: plan.group_affiliation.execution_interval,
            timeout: plan.timeout,
            n_attempts_max: plan.robot.n_attempts_max,
        },
        metadata: plan.metadata.clone(),
        machine_id: plan.machine_id.clone(),
    })
}

fn resource_attributes(
    metadata: &PlanMetadata,
    machine_id: Option<&str>,
    plan_run_id: &str,
) -> Vec<(&'static str, String)> {
    let mut attributes = vec![
        ("cmk.application", metadata.application.clone()),
        ("cmk.suite", metadata.suite_name.clone()),
        ("cmk.variant", metadata.variant.clone()),
    ];
    if let Some(machine_id) = machine_id {
        attributes.push(("cmk.machine_id", machine_id.to_string()));
    }
    attributes.push(("cmk.plan_run_id", plan_run_id.to_string()));
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_attributes_with_machine_id() {
        assert_eq!(
            resource_attributes(
                &PlanMetadata {
                    application: "app".into(),
                    suite_name: "suite".into(),
                    variant: "".into(),
                },
                Some("machine"),
                "run"
            ),
            [
                ("cmk.application", "app".to_string()),
                ("cmk.suite", "suite".to_string()),
                ("cmk.variant", "".to_string()),
                ("cmk.machine_id", "machine".to_string()),
                ("cmk.plan_run_id", "run".to_string()),
            ]
        );
    }

    #[test]
    fn resource_attributes_without_machine_id() {
        assert_eq!(
            resource_attributes(
                &PlanMetadata {
                    application: "app".into(),
                    suite_name: "suite".into(),
                    variant: "variant".into(),
                },
                None,
                "run"
            ),
            [
                ("cmk.application", "app".to_string()),
                ("cmk.suite", "suite".to_string()),
                ("cmk.variant", "variant".to_string()),
                ("cmk.plan_run_id", "run".to_string()),
            ]
        );
    }

    #[test]
    fn format_source_for_logging_manual() {
        assert_eq!(&format_source_for_logging(&Source::Manual), "manual robot");
    }

    #[test]
    fn format_source_for_logging_managed_without_version_label() {
        assert_eq!(
            &format_source_for_logging(&Source::Managed {
                tar_gz_path: "robot.tar.gz".into(),
                target: "robot".into(),
                version_number: 3,
                version_label: "".into()
            }),
            "managed robot, version: 3"
        );
    }

    #[test]
    fn format_source_for_logging_managed_with_version_label() {
        assert_eq!(
            &format_source_for_logging(&Source::Managed {
                tar_gz_path: "robot.tar.gz".into(),
                target: "robot".into(),
                version_number: 4,
                version_label: "version_label".into()
            }),
            "managed robot, version: 4 (version_label)"
        );
    }
}
