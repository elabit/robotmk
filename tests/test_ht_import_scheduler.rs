#![cfg(unix)]
pub mod helper;
use crate::helper::{await_plan_results, directory_entries};
use anyhow::{bail, Result as AnyhowResult};
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    Config, EnvironmentConfig, ExecutionConfig, PlanConfig, PlanMetadata, RCCConfig,
    RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup,
    SessionConfig, Source, WorkingDirectoryCleanupConfig,
};
use robotmk::section::Host;
use serde_json::to_string;
use std::env::var;
use std::fs::{create_dir_all, remove_file, write};
use std::time::Duration;
use tokio::{
    process::Command,
    select,
    time::{sleep, timeout},
};

#[tokio::test]
#[ignore]
async fn test_ht_import_scheduler() -> AnyhowResult<()> {
    let test_dir = Utf8PathBuf::from(var("TEST_DIR")?);
    let rcc_binary = Utf8PathBuf::from(var("RCC_BINARY_PATH")?);
    let suite_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
        .join("tests")
        .join("minimal_suite");
    create_dir_all(&test_dir)?;

    let mut rcc_task_script = Command::new(rcc_binary.clone());
    rcc_task_script
        .arg("task")
        .arg("script")
        .arg("--robot")
        .arg(suite_dir.join("robot.yaml"))
        .arg("--")
        .arg("true");
    assert!(rcc_task_script.status().await?.success());

    let mut rcc_ht_export = Command::new(rcc_binary.clone());
    rcc_ht_export
        .arg("holotree")
        .arg("export")
        .arg("--robot")
        .arg(suite_dir.join("robot.yaml"))
        .arg("--zipfile")
        .arg(test_dir.join("hololib.zip"));
    assert!(rcc_ht_export.status().await?.success());

    let mut rcc_cleanup = Command::new(rcc_binary.clone());
    rcc_cleanup.arg("configuration").arg("cleanup").arg("--all");
    assert!(rcc_cleanup.status().await?.success());

    let config = create_config(
        &test_dir,
        &suite_dir,
        RCCConfig {
            binary_path: rcc_binary,
            profile_config: RCCProfileConfig::Default,
        },
    );

    run_scheduler(
        &test_dir,
        &config,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await?;

    assert_working_directory(&config.working_directory).await?;
    assert_results_directory(&config.results_directory);
    Ok(())
}

fn create_config(test_dir: &Utf8Path, suite_dir: &Utf8Path, rcc_config: RCCConfig) -> Config {
    Config {
        working_directory: test_dir.join("working"),
        results_directory: test_dir.join("results"),
        managed_directory: test_dir.join("managed_robots"),
        rcc_config,
        plan_groups: vec![SequentialPlanGroup {
            plans: vec![PlanConfig {
                id: "rcc_headless".into(),
                source: Source::Manual {
                    base_dir: suite_dir.into(),
                },
                robot_config: RobotConfig {
                    robot_target: "tasks.robot".into(),
                    top_level_suite_name: None,
                    suites: vec![],
                    tests: vec![],
                    test_tags_include: vec![],
                    test_tags_exclude: vec![],
                    variables: vec![],
                    variable_files: vec![],
                    argument_files: vec![],
                    exit_on_failure: false,
                },
                execution_config: ExecutionConfig {
                    n_attempts_max: 1,
                    retry_strategy: RetryStrategy::Complete,
                    timeout: 10,
                },
                environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                    robot_yaml_path: "robot.yaml".into(),
                    build_timeout: 1200,
                    remote_origin: None,
                    catalog_zip: Some(test_dir.join("hololib.zip")),
                }),
                session_config: SessionConfig::Current,
                working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(4),
                host: Host::Source,
                metadata: PlanMetadata {
                    application: "app".into(),
                    suite_name: "minimal_suite".into(),
                    variant: "".into(),
                },
            }],
            execution_interval: 30,
        }],
    }
}

async fn run_scheduler(
    test_dir: &Utf8Path,
    config: &Config,
    n_seconds_run_max: u64,
) -> AnyhowResult<()> {
    let config_path = test_dir.join("config.json");
    write(&config_path, to_string(&config)?)?;
    let run_flag_path = test_dir.join("run_flag");
    write(&run_flag_path, "")?;

    let robotmk_shell_script = format!(
        "sudo unshare --net -- ping -c 1 github.com && echo 'I still have internet access' \
        || {} -vv --run-flag {run_flag_path} {config_path}",
        cargo_bin("robotmk_scheduler").to_str().unwrap()
    );
    let mut robotmk_no_env_cmd = Command::new("sh");
    robotmk_no_env_cmd.arg("-c").arg(robotmk_shell_script);
    let mut robotmk_child_proc = robotmk_no_env_cmd.spawn()?;

    select! {
        _ = await_plan_results(config) => {},
        _ = robotmk_child_proc.wait() => {
            bail!("Scheduler terminated unexpectedly")
        },
        _ = sleep(Duration::from_secs(n_seconds_run_max)) => {
            if let Err(e) = remove_file(&run_flag_path) {
                eprintln!("Removing run file failed: {e}");
            }
            bail!(format!("No plan result files appeared with {n_seconds_run_max} seconds"))
        },
    };
    remove_file(&run_flag_path)?;
    assert!(timeout(Duration::from_secs(3), robotmk_child_proc.wait())
        .await
        .is_ok());

    Ok(())
}

async fn assert_working_directory(working_directory: &Utf8Path) -> AnyhowResult<()> {
    assert!(working_directory.is_dir());
    assert_eq!(
        directory_entries(working_directory, 1),
        ["environment_building", "plans", "rcc_setup"]
    );
    assert_eq!(
        directory_entries(working_directory.join("environment_building"), 2),
        [
            "rcc_headless",
            "rcc_headless/build.stderr",
            "rcc_headless/build.stdout",
            "rcc_headless/import.stderr",
            "rcc_headless/import.stdout",
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("plans"), 1),
        ["rcc_headless"]
    );

    let entries_rcc_headless =
        directory_entries(working_directory.join("plans").join("rcc_headless"), 2).join("");
    assert!(entries_rcc_headless.contains("rebot.xml"));
    assert!(!entries_rcc_headless.contains("1.bat"));

    Ok(())
}

fn assert_results_directory(results_directory: &Utf8Path) {
    assert!(results_directory.is_dir());
    assert_eq!(
        directory_entries(results_directory, 2),
        [
            "environment_build_states.json",
            "plans",
            "plans/rcc_headless.json",
            "scheduler_phase.json",
            "setup_failures.json"
        ]
    );
}
