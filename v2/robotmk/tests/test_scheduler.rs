use anyhow::Result;
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    Config, EnvironmentConfig, ExecutionConfig, RCCConfig, RCCEnvironmentConfig, RCCProfileConfig,
    RetryStrategy, RobotFrameworkConfig, SessionConfig, SuiteConfig, UserSessionConfig,
    WorkingDirectoryCleanupConfig,
};
use robotmk::section::Host;
use serde_json::to_string;
use std::env::var;
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, write};
use std::path::Path;
use std::time::Duration;
use tokio::{process::Command, time::timeout};
use walkdir::WalkDir;

#[tokio::test]
#[ignore]
async fn test_scheduler() -> Result<()> {
    let test_dir = Utf8PathBuf::from(var("TEST_DIR")?);
    create_dir_all(&test_dir)?;
    let current_user_name = var("UserName")?;
    let config = create_config(
        &test_dir,
        &Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("minimal_suite"),
        RCCConfig {
            binary_path: var("RCC_BINARY_PATH")?.into(),
            profile_config: Some(create_rcc_profile(&test_dir)?),
        },
        &current_user_name,
    );

    run_scheduler(&test_dir, &config, var("RUN_FOR")?.parse::<u64>()?).await?;

    assert_working_directory(&config.working_directory, &current_user_name).await?;
    assert_results_directory(&config.results_directory);
    assert_rcc(&config.rcc_config).await?;

    Ok(())
}

fn create_rcc_profile(test_dir: &Utf8Path) -> Result<RCCProfileConfig> {
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
    Ok(RCCProfileConfig {
        name: "Robotmk".into(),
        path: rcc_profile_path,
    })
}

fn create_config(
    test_dir: &Utf8Path,
    suite_dir: &Utf8Path,
    rcc_config: RCCConfig,
    user_name_headed: &str,
) -> Config {
    Config {
        working_directory: test_dir.join("working"),
        results_directory: test_dir.join("results"),
        rcc_config,
        suites: [
            (
                String::from("rcc_headless"),
                SuiteConfig {
                    robot_framework_config: RobotFrameworkConfig {
                        robot_target: suite_dir.join("tasks.robot"),
                        command_line_args: vec![],
                    },
                    execution_config: ExecutionConfig {
                        n_attempts_max: 1,
                        retry_strategy: RetryStrategy::Complete,
                        execution_interval_seconds: 30,
                        timeout: 10,
                    },
                    environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                        robot_yaml_path: suite_dir.join("robot.yaml"),
                        build_timeout: 1200,
                        env_json_path: None,
                    }),
                    session_config: SessionConfig::Current,
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(
                        4,
                    ),
                    host: Host::Source,
                },
            ),
            (
                String::from("rcc_headed"),
                SuiteConfig {
                    robot_framework_config: RobotFrameworkConfig {
                        robot_target: suite_dir.join("tasks.robot"),
                        command_line_args: vec![],
                    },
                    execution_config: ExecutionConfig {
                        n_attempts_max: 1,
                        retry_strategy: RetryStrategy::Complete,
                        execution_interval_seconds: 45,
                        timeout: 15,
                    },
                    environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                        robot_yaml_path: suite_dir.join("robot.yaml"),
                        build_timeout: 1200,
                        env_json_path: None,
                    }),
                    session_config: SessionConfig::SpecificUser(UserSessionConfig {
                        user_name: user_name_headed.into(),
                    }),
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                        120,
                    ),
                    host: Host::Source,
                },
            ),
            // Note: For our test, it doesn't matter if the suite can be executed on the target
            // system. We are not checking for success. So even on systems with no Python, the test
            // will succeed.
            (
                String::from("no_rcc"),
                SuiteConfig {
                    robot_framework_config: RobotFrameworkConfig {
                        robot_target: suite_dir.join("tasks.robot"),
                        command_line_args: vec![],
                    },
                    execution_config: ExecutionConfig {
                        n_attempts_max: 1,
                        retry_strategy: RetryStrategy::Complete,
                        execution_interval_seconds: 37,
                        timeout: 17,
                    },
                    environment_config: EnvironmentConfig::System,
                    session_config: SessionConfig::Current,
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(
                        4,
                    ),
                    host: Host::Piggyback("oink".into()),
                },
            ),
        ]
        .into(),
    }
}

async fn run_scheduler(test_dir: &Utf8Path, config: &Config, n_seconds_run: u64) -> Result<()> {
    let config_path = test_dir.join("config.json");
    write(&config_path, to_string(&config)?)?;
    let run_flag_path = test_dir.join("run_flag");
    write(&run_flag_path, "")?;

    let mut robotmk_cmd = Command::new(cargo_bin("robotmk"));
    robotmk_cmd
        .arg(config_path)
        .arg("-vv")
        .arg("--run-flag")
        .arg(&run_flag_path);
    let mut robotmk_child_proc = robotmk_cmd.spawn()?;

    assert!(timeout(
        Duration::from_secs(n_seconds_run),
        robotmk_child_proc.wait()
    )
    .await
    .is_err());
    remove_file(&run_flag_path)?;
    assert!(timeout(Duration::from_secs(3), robotmk_child_proc.wait())
        .await
        .is_ok());

    Ok(())
}

async fn assert_working_directory(
    working_directory: &Utf8Path,
    headed_user_name: &str,
) -> Result<()> {
    assert_permissions(&working_directory, "BUILTIN\\Users:(OI)(CI)(F)").await?;
    assert!(working_directory.is_dir());
    assert_eq!(
        directory_entries(working_directory, 1),
        ["environment_building_stdio", "rcc_setup", "suites"]
    );
    assert_eq!(
        directory_entries(working_directory.join("rcc_setup"), 1),
        [
            "holotree_initialization_current_user.stderr",
            "holotree_initialization_current_user.stdout",
            &format!("holotree_initialization_user_{headed_user_name}.bat"),
            &format!("holotree_initialization_user_{headed_user_name}.exit_code"),
            &format!("holotree_initialization_user_{headed_user_name}.pid"),
            &format!("holotree_initialization_user_{headed_user_name}.run_flag",),
            &format!("holotree_initialization_user_{headed_user_name}.stderr"),
            &format!("holotree_initialization_user_{headed_user_name}.stdout"),
            "long_path_support_enabling.stderr",
            "long_path_support_enabling.stdout",
            "profile_import_current_user.stderr",
            "profile_import_current_user.stdout",
            &format!("profile_import_user_{headed_user_name}.bat"),
            &format!("profile_import_user_{headed_user_name}.exit_code"),
            &format!("profile_import_user_{headed_user_name}.pid"),
            &format!("profile_import_user_{headed_user_name}.run_flag"),
            &format!("profile_import_user_{headed_user_name}.stderr"),
            &format!("profile_import_user_{headed_user_name}.stdout"),
            "profile_switch_current_user.stderr",
            "profile_switch_current_user.stdout",
            &format!("profile_switch_user_{headed_user_name}.bat"),
            &format!("profile_switch_user_{headed_user_name}.exit_code"),
            &format!("profile_switch_user_{headed_user_name}.pid"),
            &format!("profile_switch_user_{headed_user_name}.run_flag"),
            &format!("profile_switch_user_{headed_user_name}.stderr"),
            &format!("profile_switch_user_{headed_user_name}.stdout"),
            "shared_holotree_init.stderr",
            "shared_holotree_init.stdout",
            "telemetry_disabling_current_user.stderr",
            "telemetry_disabling_current_user.stdout",
            &format!("telemetry_disabling_user_{headed_user_name}.bat"),
            &format!("telemetry_disabling_user_{headed_user_name}.exit_code"),
            &format!("telemetry_disabling_user_{headed_user_name}.pid"),
            &format!("telemetry_disabling_user_{headed_user_name}.run_flag"),
            &format!("telemetry_disabling_user_{headed_user_name}.stderr"),
            &format!("telemetry_disabling_user_{headed_user_name}.stdout")
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("environment_building_stdio"), 1),
        [
            "rcc_headed.stderr",
            "rcc_headed.stdout",
            "rcc_headless.stderr",
            "rcc_headless.stdout"
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("suites"), 1),
        ["no_rcc", "rcc_headed", "rcc_headless"]
    );

    // We expliclitly don't check for the rebot files in the case without RCC, since this must also
    // work on systems that don't have the necessary Python environment.
    assert!(!directory_entries(working_directory.join("suites").join("no_rcc"), 1).is_empty());

    let entries_rcc_headed =
        directory_entries(working_directory.join("suites").join("rcc_headed"), 2).join("");
    assert!(entries_rcc_headed.contains("rebot.xml"));
    assert!(entries_rcc_headed.contains("0.bat"));

    let entries_rcc_headless =
        directory_entries(working_directory.join("suites").join("rcc_headless"), 2).join("");
    assert!(entries_rcc_headless.contains("rebot.xml"));
    assert!(!entries_rcc_headless.contains("0.bat"));

    Ok(())
}

async fn assert_permissions(path: impl AsRef<OsStr>, permissions: &str) -> Result<()> {
    let mut icacls_command = Command::new("icacls.exe");
    icacls_command.arg(path);
    assert!(String::from_utf8(icacls_command.output().await?.stdout)?.contains(permissions));
    Ok(())
}

fn assert_results_directory(results_directory: &Utf8Path) {
    assert!(results_directory.is_dir());
    assert_eq!(
        directory_entries(results_directory, 2),
        [
            "environment_build_states.json",
            "rcc_setup_failures.json",
            "scheduler_phase.json",
            "suites",
            "suites\\no_rcc.json",
            "suites\\rcc_headed.json",
            "suites\\rcc_headless.json"
        ]
    );
}

async fn assert_rcc(rcc_config: &RCCConfig) -> Result<()> {
    assert_rcc_files_permissions(rcc_config).await?;
    assert_rcc_configuration(rcc_config).await?;
    assert_rcc_longpath_support_enabled(&rcc_config.binary_path).await
}

async fn assert_rcc_files_permissions(rcc_config: &RCCConfig) -> Result<()> {
    assert_permissions(&rcc_config.binary_path, "BUILTIN\\Users:(RX)").await?;
    let Some(rcc_profile_config) = &rcc_config.profile_config else {
        return Ok(());
    };
    assert_permissions(&rcc_profile_config.path, "BUILTIN\\Users:(R)").await
}

async fn assert_rcc_configuration(rcc_config: &RCCConfig) -> Result<()> {
    let mut rcc_config_diag_command = Command::new(&rcc_config.binary_path);
    rcc_config_diag_command
        .arg("configuration")
        .arg("diagnostics");
    let stdout = String::from_utf8(rcc_config_diag_command.output().await?.stdout)?;
    assert!(stdout.contains("telemetry-enabled                     ...  \"false\""));
    assert!(stdout.contains("holotree-shared                       ...  \"true\""));
    assert!(stdout.contains("holotree-global-shared                ...  \"true\""));
    if let Some(rcc_profile_config) = &rcc_config.profile_config {
        assert!(stdout.contains(&format!(
            "config-active-profile                 ...  \"{}\"",
            rcc_profile_config.name
        )));
    }
    Ok(())
}

async fn assert_rcc_longpath_support_enabled(rcc_binary_path: impl AsRef<OsStr>) -> Result<()> {
    let mut rcc_config_diag_command = Command::new(rcc_binary_path);
    rcc_config_diag_command
        .arg("configuration")
        .arg("longpaths");
    let stderr = String::from_utf8(rcc_config_diag_command.output().await?.stderr)?;
    assert_eq!(stderr, "OK.\n");
    Ok(())
}

fn directory_entries(directory: impl AsRef<Path>, max_depth: usize) -> Vec<String> {
    WalkDir::new(&directory)
        .max_depth(max_depth)
        .sort_by_file_name()
        .into_iter()
        .map(|dir_entry_result| {
            dir_entry_result
                .unwrap()
                .path()
                .strip_prefix(&directory)
                .unwrap()
                .to_str()
                .unwrap()
                .into()
        })
        .filter(|entry: &String| !entry.is_empty())
        .collect()
}
