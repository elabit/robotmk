#![cfg(unix)]
pub mod helper;
use crate::helper::{await_plan_results, directory_entries, var};
use anyhow::{Result as AnyhowResult, bail};
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    CondaConfig, CondaEnvironmentConfig, CondaEnvironmentSource, Config, EnvironmentConfig,
    ExecutionConfig, HTTPProxyConfig, PlanConfig, PlanMetadata, RCCConfig, RCCProfileConfig,
    RetryStrategy, RobotConfig, SequentialPlanGroup, SessionConfig, Source,
    TlsCertificateValidation, WorkingDirectoryCleanupConfig,
};
use robotmk::results::results_directory;
use robotmk::section::Host;
use serde_json::to_string;
use std::fs::{create_dir_all, remove_file, write};
use std::time::Duration;
use tempfile::tempdir;
use tokio::{
    process::Command,
    select,
    time::{sleep, timeout},
};

#[tokio::test]
#[ignore]
async fn test_conda_from_archive_scheduler() -> AnyhowResult<()> {
    let test_dir = Utf8PathBuf::from(var("TEST_DIR")?);
    let micromamba_binary_path = Utf8PathBuf::from(var("MICROMAMBA_BINARY_PATH")?);
    let suite_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
        .join("tests")
        .join("minimal_suite");
    create_dir_all(&test_dir)?;

    let temp_dir = tempdir()?;
    let conda_prefix = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?.join("env");
    let env_archive_path = test_dir.join("env.tar.gz");
    let runtime_dir = test_dir.join("runtime");
    let plan_id = "plan1";

    let mut create_cmd = Command::new(&micromamba_binary_path);
    create_cmd
        .arg("create")
        .arg("--file")
        .arg(suite_dir.join("conda.yaml"))
        .arg("--prefix")
        .arg(&conda_prefix)
        .arg("--yes");
    assert!(create_cmd.status().await?.success());

    let mut pack_cmd = Command::new(&micromamba_binary_path);
    pack_cmd
        .arg("run")
        .arg("--prefix")
        .arg(&conda_prefix)
        .arg("conda-pack")
        .arg("--prefix")
        .arg(&conda_prefix)
        .arg("--output")
        .arg(&env_archive_path);
    assert!(pack_cmd.status().await?.success());
    assert!(env_archive_path.is_file());

    let config = create_config(
        &runtime_dir,
        &suite_dir,
        CondaConfig {
            micromamba_binary_path,
            base_directory: test_dir.join("conda_base"),
        },
        plan_id,
        &env_archive_path,
    );

    run_scheduler(
        &test_dir,
        &config,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await?;

    assert_working_directory(&config.runtime_directory.join("working"), plan_id).await?;
    assert_results_directory(&results_directory(&config.runtime_directory), plan_id);
    Ok(())
}

fn create_config(
    runtime_dir: &Utf8Path,
    suite_dir: &Utf8Path,
    conda_config: CondaConfig,
    plan_id: &str,
    packed_conda_env_path: &Utf8Path,
) -> Config {
    Config {
        runtime_directory: runtime_dir.into(),
        rcc_config: RCCConfig {
            binary_path: Utf8PathBuf::default(),
            profile_config: RCCProfileConfig::Default,
            robocorp_home_base: Utf8PathBuf::default(),
        },
        conda_config,
        plan_groups: vec![SequentialPlanGroup {
            plans: vec![PlanConfig {
                id: plan_id.into(),
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
                    source: CondaEnvironmentSource::Archive(packed_conda_env_path.into()),
                    robotmk_manifest_path: None,
                    http_proxy_config: HTTPProxyConfig::default(),
                    tls_certificate_validation: TlsCertificateValidation::Enabled,
                    tls_revokation_enabled: false,
                    build_timeout: 1200,
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
            bail!(format!("Not all plan result files appeared within {n_seconds_run_max} seconds"))
        },
    };
    remove_file(&run_flag_path)?;
    assert!(
        timeout(Duration::from_secs(3), robotmk_child_proc.wait())
            .await
            .is_ok()
    );

    Ok(())
}

async fn assert_working_directory(working_directory: &Utf8Path, plan_id: &str) -> AnyhowResult<()> {
    assert!(working_directory.is_dir());
    assert_eq!(
        directory_entries(working_directory, 1),
        ["environment_building", "plans", "rcc_setup"]
    );
    assert_eq!(
        directory_entries(working_directory.join("environment_building"), 2),
        [
            plan_id,
            &format!("{plan_id}/conda-unpack.stderr"),
            &format!("{plan_id}/conda-unpack.stdout"),
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("plans"), 1),
        [plan_id]
    );

    let entries_plan_working_dir =
        directory_entries(working_directory.join("plans").join(plan_id), 2).join("");
    assert!(entries_plan_working_dir.contains("rebot.xml"));
    assert!(!entries_plan_working_dir.contains("1.bat"));

    Ok(())
}

fn assert_results_directory(results_directory: &Utf8Path, plan_id: &str) {
    assert!(results_directory.is_dir());
    assert_eq!(
        directory_entries(results_directory, 2),
        [
            "environment_build_states.json",
            "plans",
            &format!("plans/{plan_id}.json"),
            "scheduler_phase.json",
            "setup_failures.json"
        ]
    );
}
