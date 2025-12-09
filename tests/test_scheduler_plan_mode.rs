pub mod helper;
pub mod rcc;
use crate::helper::var;
use anyhow::{Result as AnyhowResult, bail};
use assert_cmd::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    CondaConfig, CondaEnvironmentConfig, CondaEnvironmentSource, Config, CustomRCCProfileConfig,
    EnvironmentConfig, ExecutionConfig, HTTPProxyConfig, PlanConfig, PlanMetadata, RCCConfig,
    RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup, SessionConfig, Source,
    TlsCertificateValidation, WorkingDirectoryCleanupConfig,
};
use robotmk::results::{plan_results_directory, results_directory};
use robotmk::section::Host;
use serde_json::to_string;
use std::fs::{create_dir_all, write};
use std::time::Duration;
use tokio::{process::Command, time::timeout};

fn setup_test_environment() -> AnyhowResult<(Utf8PathBuf, Config)> {
    let test_dir = Utf8PathBuf::from(var("TEST_DIR")?);

    if test_dir.exists() {
        std::fs::remove_dir_all(&test_dir)?;
    }
    create_dir_all(&test_dir)?;

    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path: Utf8PathBuf = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;

    let suite_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
        .join("tests")
        .join("minimal_suite");

    let config = create_config(
        &test_dir,
        &suite_dir,
        RCCConfig {
            binary_path: var("RCC_BINARY_PATH")?.into(),
            profile_config: RCCProfileConfig::Custom(create_custom_rcc_profile(&test_dir)?),
            robocorp_home_base: temp_dir_path.join("rc_home_base"),
        },
        CondaConfig {
            micromamba_binary_path: var("MICROMAMBA_BINARY_PATH")?.into(),
            base_directory: temp_dir_path.join("conda_base"),
        },
    );

    Ok((test_dir, config))
}

#[tokio::test]
#[ignore]
async fn test_plan_mode() -> AnyhowResult<()> {
    let (test_dir, config) = setup_test_environment()?;

    run_scheduler(
        &test_dir,
        &config,
        "random_plan",
        false,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await?;

    let results_dir = plan_results_directory(&results_directory(&config.runtime_directory));
    assert!(results_dir.is_dir(), "Plan results directory should exist");

    let random_plan_result = results_dir.join("random_plan.json");
    assert!(
        random_plan_result.is_file(),
        "random_plan.json result file should exist"
    );

    let conda_plan_result = results_dir.join("conda_plan.json");
    assert!(
        !conda_plan_result.is_file(),
        "conda_plan.json result file should NOT exist (only random_plan was executed)"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_plan_mode_no_result() -> AnyhowResult<()> {
    let (test_dir, config) = setup_test_environment()?;

    run_scheduler(
        &test_dir,
        &config,
        "random_plan",
        true,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await?;

    let results_dir = plan_results_directory(&results_directory(&config.runtime_directory));
    let random_plan_result = results_dir.join("random_plan.json");

    assert!(
        !random_plan_result.is_file(),
        "random_plan.json result file should NOT exist when --no-plan-result is specified"
    );

    let working_dir = config
        .runtime_directory
        .join("working")
        .join("plans")
        .join("random_plan");
    assert!(
        working_dir.is_dir(),
        "Plan working directory should exist, indicating the plan was executed"
    );

    let execution_dirs: Vec<_> = std::fs::read_dir(&working_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect();
    assert_eq!(
        execution_dirs.len(),
        1,
        "There should be exactly one execution subdirectory"
    );

    let execution_dir = &execution_dirs[0].path();
    let execution_dir_entries: Vec<_> = std::fs::read_dir(execution_dir)?
        .filter_map(|entry| entry.ok())
        .collect();
    assert!(
        !execution_dir_entries.is_empty(),
        "Execution directory should be non-empty"
    );

    Ok(())
}

fn create_custom_rcc_profile(test_dir: &Utf8Path) -> AnyhowResult<CustomRCCProfileConfig> {
    let rcc_profile_path = test_dir.join("rcc_profile.yaml");
    write(
        &rcc_profile_path,
        "name: Robotmk
description: Robotmk RCC profile
settings:
  meta:
    name: Robotmk
    description: Robotmk RCC profile
    source: Robotmk
",
    )?;
    Ok(CustomRCCProfileConfig {
        name: "Robotmk".into(),
        path: rcc_profile_path,
    })
}

fn create_config(
    runtime_dir: &Utf8Path,
    suite_dir: &Utf8Path,
    rcc_config: RCCConfig,
    conda_config: CondaConfig,
) -> Config {
    Config {
        runtime_directory: runtime_dir.into(),
        rcc_config,
        conda_config,
        plan_groups: vec![SequentialPlanGroup {
            plans: vec![
                PlanConfig {
                    id: "random_plan".into(),
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
                        environment_variables_rendered_obfuscated: vec![],
                    },
                    execution_config: ExecutionConfig {
                        n_attempts_max: 1,
                        retry_strategy: RetryStrategy::Complete,
                        timeout: 10,
                    },
                    environment_config: EnvironmentConfig::Conda(CondaEnvironmentConfig {
                        source: CondaEnvironmentSource::Manifest("conda.yaml".into()),
                        robotmk_manifest_path: None,
                        http_proxy_config: HTTPProxyConfig::default(),
                        tls_certificate_validation: TlsCertificateValidation::Enabled,
                        tls_revokation_enabled: true,
                        build_timeout: 1200,
                    }),
                    session_config: SessionConfig::Current,
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(
                        4,
                    ),
                    host: Host::Source,
                    metadata: PlanMetadata {
                        application: "app".into(),
                        suite_name: "minimal_suite".into(),
                        variant: "".into(),
                    },
                },
                PlanConfig {
                    id: "conda_plan".into(),
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
                        environment_variables_rendered_obfuscated: vec![],
                    },
                    execution_config: ExecutionConfig {
                        n_attempts_max: 1,
                        retry_strategy: RetryStrategy::Complete,
                        timeout: 10,
                    },
                    environment_config: EnvironmentConfig::Conda(CondaEnvironmentConfig {
                        source: CondaEnvironmentSource::Manifest("conda.yaml".into()),
                        robotmk_manifest_path: None,
                        http_proxy_config: HTTPProxyConfig::default(),
                        tls_certificate_validation: TlsCertificateValidation::Enabled,
                        tls_revokation_enabled: true,
                        build_timeout: 1200,
                    }),
                    session_config: SessionConfig::Current,
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(
                        4,
                    ),
                    host: Host::Source,
                    metadata: PlanMetadata {
                        application: "app".into(),
                        suite_name: "minimal_suite".into(),
                        variant: "".into(),
                    },
                },
            ],
            execution_interval: 30,
        }],
    }
}

async fn run_scheduler(
    test_dir: &Utf8Path,
    config: &Config,
    run_plan_only: &str,
    no_plan_result: bool,
    n_seconds_run_max: u64,
) -> AnyhowResult<()> {
    let config_path = test_dir.join("config.json");
    write(&config_path, to_string(&config)?)?;

    let mut robotmk_cmd = Command::new(cargo_bin!("robotmk_scheduler"));
    robotmk_cmd
        .arg(config_path)
        .arg("-vv")
        .arg("--plan")
        .arg(run_plan_only);

    if no_plan_result {
        robotmk_cmd.arg("--no-plan-result");
    }

    let mut robotmk_child_proc = robotmk_cmd.spawn()?;

    let exit_status = timeout(
        Duration::from_secs(n_seconds_run_max),
        robotmk_child_proc.wait(),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Scheduler did not complete within {} seconds",
            n_seconds_run_max
        )
    })??;

    if !exit_status.success() {
        bail!("Scheduler exited with non-zero status: {}", exit_status);
    }

    Ok(())
}
