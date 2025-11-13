mod build;
mod cli;
mod internal_config;
mod logging;
mod scheduling;
mod setup;
mod termination;

use anyhow::{Context, Result as AnyhowResult};
use clap::Parser;
use log::info;
use logging::log_and_return_error;
use robotmk::config::Config;
use robotmk::lock::Locker;
use robotmk::results::{SchedulerPhase, SetupFailure, SetupFailures};
use robotmk::section::WriteSection;
use robotmk::termination::Terminate;
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use tokio_util::sync::CancellationToken;

fn main() -> AnyhowResult<()> {
    if let Err(e) = run() {
        return match e {
            Terminate::Cancelled => {
                info!("Terminated");
                Ok(())
            }
            Terminate::Unrecoverable(any) => Err(log_and_return_error(any)),
        };
    }
    Ok(())
}

fn run() -> Result<(), Terminate> {
    let args = cli::Args::parse();
    logging::init(args.log_specification(), args.log_path).context("Logging setup failed.")?;
    info!("Program started and logging set up");

    let external_config = filter_by_plan_id(
        robotmk::config::load(&args.config_path).context("Configuration loading failed")?,
        args.plan.as_deref(),
    )?;
    info!("Configuration loaded");

    let cancellation_token = termination::start_termination_control(args.run_flag)
        .context("Failed to set up termination control")?;
    info!("Termination control set up");

    let (global_config, plans) = internal_config::from_external_config(
        external_config,
        &cancellation_token,
        &Locker::new(&args.config_path, Some(&cancellation_token)),
    );

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    if let Some(grace_period) = args.grace_period {
        info!("Grace period: Sleeping for {grace_period} seconds");
        write_phase(&SchedulerPhase::GracePeriod(grace_period), &global_config)?;
        await_grace_period(grace_period, &cancellation_token);
    }

    setup::base_directories::setup(&global_config, &plans)?;
    info!("Base setup completed");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    write_phase(&SchedulerPhase::Setup, &global_config)?;
    let (plans, setup_failures) = setup::steps::run::run(&global_config, plans)?;
    write_setup_failures(setup_failures.into_iter(), &global_config)?;
    info!("Setup steps completed");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    info!("Starting environment building");
    write_phase(&SchedulerPhase::EnvironmentBuilding, &global_config)?;
    let plans = build::build_environments(&global_config, plans)?;
    info!("Environment building finished");

    if global_config.cancellation_token.is_cancelled() {
        return Err(Terminate::Cancelled);
    }

    info!("Starting plan scheduling");
    write_phase(&SchedulerPhase::Scheduling, &global_config)?;
    if args.plan.is_some() {
        if let Some(plan) = plans.first() {
            scheduling::plans::run_plan(plan)?;
        }
    } else {
        scheduling::scheduler::run_plans_and_cleanup(&global_config, &plans);
    }
    Err(Terminate::Cancelled)
}

fn write_phase(
    phase: &SchedulerPhase,
    global_config: &internal_config::GlobalConfig,
) -> Result<(), Terminate> {
    phase.write(
        global_config.results_directory.join("scheduler_phase.json"),
        &global_config.results_directory_locker,
    )
}

fn write_setup_failures(
    failures: impl Iterator<Item = SetupFailure>,
    global_config: &internal_config::GlobalConfig,
) -> Result<(), Terminate> {
    SetupFailures(failures.collect()).write(
        global_config.results_directory.join("setup_failures.json"),
        &global_config.results_directory_locker,
    )
}

fn filter_by_plan_id(mut config: Config, plan_id: Option<&str>) -> Result<Config, Terminate> {
    if let Some(plan_id) = plan_id {
        info!("Filtering configuration to only include plan: {}", plan_id);

        config.plan_groups.retain_mut(|group| {
            group.plans.retain(|p| p.id == plan_id);
            !group.plans.is_empty()
        });
        if !config.plan_groups.iter().any(|g| !g.plans.is_empty()) {
            return Err(Terminate::Unrecoverable(anyhow::anyhow!(
                "No plans found matching id '{}'",
                plan_id
            )));
        }
    }
    Ok(config)
}

#[tokio::main]
async fn await_grace_period(grace_period: u64, cancellation_token: &CancellationToken) {
    let _ = timeout_at(
        Instant::now() + Duration::from_secs(grace_period),
        cancellation_token.cancelled(),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use robotmk::config::{
        CondaConfig, EnvironmentConfig, ExecutionConfig, PlanConfig, PlanMetadata, RCCConfig,
        RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup, SessionConfig, Source,
        WorkingDirectoryCleanupConfig,
    };
    use robotmk::section::Host;

    fn create_test_config() -> Config {
        Config {
            runtime_directory: Utf8PathBuf::from("/test"),
            rcc_config: RCCConfig {
                binary_path: Utf8PathBuf::from("/test/rcc"),
                profile_config: RCCProfileConfig::Default,
                robocorp_home_base: Utf8PathBuf::from("/test/rcc_home"),
            },
            conda_config: CondaConfig {
                micromamba_binary_path: Utf8PathBuf::from("/test/micromamba"),
                base_directory: Utf8PathBuf::from("/test/conda"),
            },
            plan_groups: vec![SequentialPlanGroup {
                plans: vec![
                    PlanConfig {
                        id: "plan1".to_string(),
                        source: Source::Manual {
                            base_dir: Utf8PathBuf::from("/test/plan1"),
                        },
                        robot_config: RobotConfig {
                            robot_target: Utf8PathBuf::from("tasks.robot"),
                            top_level_suite_name: None,
                            suites: vec![],
                            tests: vec![],
                            test_tags_include: vec![],
                            test_tags_exclude: vec![],
                            variables: vec![],
                            variable_files: vec![],
                            argument_files: vec![],
                            exit_on_failure: false,
                            environment_variables_rendered_obfuscated: vec![],
                        },
                        execution_config: ExecutionConfig {
                            n_attempts_max: 1,
                            retry_strategy: RetryStrategy::Complete,
                            timeout: 60,
                        },
                        environment_config: EnvironmentConfig::System,
                        session_config: SessionConfig::Current,
                        working_directory_cleanup_config:
                            WorkingDirectoryCleanupConfig::MaxExecutions(5),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "test_app".to_string(),
                            suite_name: "test_suite".to_string(),
                            variant: "".to_string(),
                        },
                    },
                    PlanConfig {
                        id: "plan2".to_string(),
                        source: Source::Manual {
                            base_dir: Utf8PathBuf::from("/test/plan2"),
                        },
                        robot_config: RobotConfig {
                            robot_target: Utf8PathBuf::from("tasks.robot"),
                            top_level_suite_name: None,
                            suites: vec![],
                            tests: vec![],
                            test_tags_include: vec![],
                            test_tags_exclude: vec![],
                            variables: vec![],
                            variable_files: vec![],
                            argument_files: vec![],
                            exit_on_failure: false,
                            environment_variables_rendered_obfuscated: vec![],
                        },
                        execution_config: ExecutionConfig {
                            n_attempts_max: 1,
                            retry_strategy: RetryStrategy::Complete,
                            timeout: 60,
                        },
                        environment_config: EnvironmentConfig::System,
                        session_config: SessionConfig::Current,
                        working_directory_cleanup_config:
                            WorkingDirectoryCleanupConfig::MaxExecutions(5),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "test_app".to_string(),
                            suite_name: "test_suite".to_string(),
                            variant: "".to_string(),
                        },
                    },
                ],
                execution_interval: 300,
            }],
        }
    }

    #[test]
    fn test_filter_by_plan_id_filters_correctly() {
        let filtered_config = filter_by_plan_id(create_test_config(), Some("plan1")).unwrap();
        assert_eq!(filtered_config.plan_groups.len(), 1);
        assert_eq!(filtered_config.plan_groups[0].plans.len(), 1);
        assert_eq!(filtered_config.plan_groups[0].plans[0].id, "plan1");
    }

    #[test]
    fn test_filter_by_plan_id_returns_unchanged_when_none() {
        let config = create_test_config();
        let original_plan_count: usize = config.plan_groups.iter().map(|g| g.plans.len()).sum();

        let filtered_config = filter_by_plan_id(config, None).unwrap();

        let filtered_plan_count: usize = filtered_config
            .plan_groups
            .iter()
            .map(|g| g.plans.len())
            .sum();
        assert_eq!(filtered_plan_count, original_plan_count);
    }

    #[test]
    fn test_filter_by_plan_id_errors_on_nonexistent_plan() {
        let result = filter_by_plan_id(create_test_config(), Some("nonexistent_plan"));
        assert!(result.is_err());
    }
}
