use crate::internal_config::{Plan, Source};
use crate::logging::TIMESTAMP_FORMAT;
use robotmk::plans::run_attempts_with_rebot;
use robotmk::results::{AttemptsConfig, PlanExecutionReport};

use anyhow::{Context, Result as AnyhowResult};
use chrono::Utc;
use log::info;
use robotmk::section::WritePiggybackSection;
use std::fs::create_dir_all;

pub fn run_plan(plan: &Plan) -> AnyhowResult<()> {
    info!(
        "Running plan {} ({})",
        &plan.id,
        format_source_for_logging(&plan.source)
    );
    produce_plan_results(plan)?
        .write(
            &plan.results_file,
            plan.host.clone(),
            &plan.results_directory_locker,
        )
        .context("Reporting plan results failed")?;
    info!("Plan {} finished", &plan.id);

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

fn produce_plan_results(plan: &Plan) -> AnyhowResult<PlanExecutionReport> {
    let timestamp = Utc::now();
    let output_directory = plan
        .working_directory
        .join(timestamp.format(TIMESTAMP_FORMAT).to_string());

    create_dir_all(&output_directory).context(format!(
        "Failed to create directory for plan run: {}",
        output_directory
    ))?;

    let (attempt_reports, rebot) = run_attempts_with_rebot(
        &plan.robot,
        &plan.id,
        &plan.environment,
        &plan.session,
        plan.timeout,
        &plan.cancellation_token,
        &output_directory,
    )
    .context("Received termination signal while running plan")?;

    Ok(PlanExecutionReport {
        plan_id: plan.id.clone(),
        timestamp: timestamp.timestamp(),
        attempts: attempt_reports,
        rebot,
        config: AttemptsConfig {
            interval: plan.group_affiliation.execution_interval,
            timeout: plan.timeout,
            n_attempts_max: plan.robot.n_attempts_max,
        },
        metadata: plan.metadata.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
