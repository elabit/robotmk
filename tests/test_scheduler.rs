pub mod rcc;
use crate::rcc::read_configuration_diagnostics;
use anyhow::Result as AnyhowResult;
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    Config, CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, PlanConfig, PlanMetadata,
    RCCConfig, RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig,
    SequentialPlanGroup, SessionConfig, Source, UserSessionConfig, WorkingDirectoryCleanupConfig,
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
async fn test_scheduler() -> AnyhowResult<()> {
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
            profile_config: RCCProfileConfig::Custom(create_custom_rcc_profile(&test_dir)?),
        },
        &current_user_name,
    );

    run_scheduler(&test_dir, &config, var("RUN_FOR")?.parse::<u64>()?).await?;

    assert_working_directory(&config.working_directory, &current_user_name).await?;
    assert_results_directory(&config.results_directory);
    assert_rcc(&config.rcc_config, &current_user_name).await?;
    assert_tasks().await?;

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
    test_dir: &Utf8Path,
    suite_dir: &Utf8Path,
    rcc_config: RCCConfig,
    user_name_headed: &str,
) -> Config {
    Config {
        working_directory: test_dir.join("working"),
        results_directory: test_dir.join("results"),
        managed_directory: test_dir.join("managed_robots"),
        rcc_config,
        plan_groups: vec![
            SequentialPlanGroup {
                plans: vec![
                    PlanConfig {
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
                        }),
                        session_config: SessionConfig::Current,
                        working_directory_cleanup_config:
                            WorkingDirectoryCleanupConfig::MaxExecutions(4),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "app1".into(),
                            suite_name: "minimal_suite".into(),
                            variant: "".into(),
                        },
                    },
                    PlanConfig {
                        id: "rcc_headed".into(),
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
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                            robot_yaml_path: "robot.yaml".into(),
                            build_timeout: 1200,
                        }),
                        session_config: SessionConfig::SpecificUser(UserSessionConfig {
                            user_name: user_name_headed.into(),
                        }),
                        working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                            120,
                        ),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "app2".into(),
                            suite_name: "minimal_suite".into(),
                            variant: "".into(),
                        },
                    },
                    PlanConfig {
                        id: "managed_robot_zip".into(),
                        source: Source::Managed {
                            zip_file: "C:\\zips\\restry_rcc_defN.zip".into(),
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
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                            robot_yaml_path: "tasks.robot".into(),
                            build_timeout: 1200,
                        }),
                        session_config: SessionConfig::SpecificUser(UserSessionConfig {
                            user_name: user_name_headed.into(),
                        }),
                        working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                            120,
                        ),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "managed".into(),
                            suite_name: "robot_zip".into(),
                            variant: "".into(),
                        },
                    },
                ],
                execution_interval: 30,
            },
            // Note: For our test, it doesn't matter if the suite can be executed on the target
            // system. We are not checking for success. So even on systems with no Python, the test
            // will succeed.
            SequentialPlanGroup {
                plans: vec![PlanConfig {
                    id: "no_rcc".into(),
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
                        timeout: 17,
                    },
                    environment_config: EnvironmentConfig::System,
                    session_config: SessionConfig::Current,
                    working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxExecutions(
                        4,
                    ),
                    host: Host::Piggyback("oink".into()),
                    metadata: PlanMetadata {
                        application: "app3".into(),
                        suite_name: "minimal_suite".into(),
                        variant: "".into(),
                    },
                }],
                execution_interval: 37,
            },
        ],
    }
}

async fn run_scheduler(
    test_dir: &Utf8Path,
    config: &Config,
    n_seconds_run: u64,
) -> AnyhowResult<()> {
    let config_path = test_dir.join("config.json");
    write(&config_path, to_string(&config)?)?;
    let run_flag_path = test_dir.join("run_flag");
    write(&run_flag_path, "")?;

    let mut robotmk_cmd = Command::new(cargo_bin("robotmk_scheduler"));
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
) -> AnyhowResult<()> {
    assert_permissions(
        &working_directory,
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    assert!(working_directory.is_dir());
    assert_eq!(
        directory_entries(working_directory, 1),
        ["environment_building", "plans", "rcc_setup"]
    );
    assert_eq!(
        directory_entries(working_directory.join("rcc_setup"), 1),
        [
            "custom_profile_import_current_user.stderr",
            "custom_profile_import_current_user.stdout",
            &format!("custom_profile_import_user_{headed_user_name}.bat"),
            &format!("custom_profile_import_user_{headed_user_name}.exit_code"),
            &format!("custom_profile_import_user_{headed_user_name}.stderr"),
            &format!("custom_profile_import_user_{headed_user_name}.stdout"),
            "custom_profile_switch_current_user.stderr",
            "custom_profile_switch_current_user.stdout",
            &format!("custom_profile_switch_user_{headed_user_name}.bat"),
            &format!("custom_profile_switch_user_{headed_user_name}.exit_code"),
            &format!("custom_profile_switch_user_{headed_user_name}.stderr"),
            &format!("custom_profile_switch_user_{headed_user_name}.stdout"),
            "holotree_initialization_current_user.stderr",
            "holotree_initialization_current_user.stdout",
            &format!("holotree_initialization_user_{headed_user_name}.bat"),
            &format!("holotree_initialization_user_{headed_user_name}.exit_code"),
            &format!("holotree_initialization_user_{headed_user_name}.stderr"),
            &format!("holotree_initialization_user_{headed_user_name}.stdout"),
            "long_path_support_enabling.stderr",
            "long_path_support_enabling.stdout",
            "telemetry_disabling_current_user.stderr",
            "telemetry_disabling_current_user.stdout",
            &format!("telemetry_disabling_user_{headed_user_name}.bat"),
            &format!("telemetry_disabling_user_{headed_user_name}.exit_code"),
            &format!("telemetry_disabling_user_{headed_user_name}.stderr"),
            &format!("telemetry_disabling_user_{headed_user_name}.stdout")
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("environment_building"), 1),
        [
            "rcc_headed.bat",
            "rcc_headed.exit_code",
            "rcc_headed.stderr",
            "rcc_headed.stdout",
            "rcc_headless.stderr",
            "rcc_headless.stdout"
        ]
    );
    assert_eq!(
        directory_entries(working_directory.join("plans"), 1),
        ["managed_robot_zip", "no_rcc", "rcc_headed", "rcc_headless"]
    );

    // We expliclitly don't check for the rebot files in the case without RCC, since this must also
    // work on systems that don't have the necessary Python environment.
    assert!(!directory_entries(working_directory.join("plans").join("no_rcc"), 1).is_empty());

    let entries_rcc_headed =
        directory_entries(working_directory.join("plans").join("rcc_headed"), 2).join("");
    assert!(entries_rcc_headed.contains("rebot.xml"));
    assert!(entries_rcc_headed.contains("1.bat"));

    let entries_rcc_headless =
        directory_entries(working_directory.join("plans").join("rcc_headless"), 2).join("");
    assert!(entries_rcc_headless.contains("rebot.xml"));
    assert!(!entries_rcc_headless.contains("1.bat"));

    Ok(())
}

async fn assert_permissions(path: impl AsRef<OsStr>, permissions: &str) -> AnyhowResult<()> {
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
            "general_setup_failures.json",
            "management_failures.json",
            "plans",
            "plans\\no_rcc.json",
            "plans\\rcc_headed.json",
            "plans\\rcc_headless.json",
            "rcc_setup_failures.json",
            "scheduler_phase.json",
        ]
    );
}

async fn assert_rcc(rcc_config: &RCCConfig, headed_user_name: &str) -> AnyhowResult<()> {
    assert_rcc_files_permissions(rcc_config, headed_user_name).await?;
    assert_rcc_configuration(rcc_config).await?;
    assert_rcc_longpath_support_enabled(&rcc_config.binary_path).await
}

async fn assert_rcc_files_permissions(
    rcc_config: &RCCConfig,
    headed_user_name: &str,
) -> AnyhowResult<()> {
    assert_permissions(&rcc_config.binary_path, &format!("{headed_user_name}:(RX)")).await?;
    let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config else {
        return Ok(());
    };
    assert_permissions(
        &custom_rcc_profile_config.path,
        &format!("{headed_user_name}:(R)"),
    )
    .await
}

async fn assert_rcc_configuration(rcc_config: &RCCConfig) -> AnyhowResult<()> {
    let diagnostics = read_configuration_diagnostics(&rcc_config.binary_path)?;
    assert_eq!(
        diagnostics
            .details
            .get("telemetry-enabled")
            .unwrap()
            .as_str(),
        "false"
    );
    if let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config {
        assert_eq!(
            diagnostics
                .details
                .get("config-active-profile")
                .unwrap()
                .as_str(),
            custom_rcc_profile_config.name
        );
    }
    Ok(())
}

async fn assert_rcc_longpath_support_enabled(
    rcc_binary_path: impl AsRef<OsStr>,
) -> AnyhowResult<()> {
    let mut rcc_config_diag_command = Command::new(rcc_binary_path);
    rcc_config_diag_command
        .arg("configuration")
        .arg("longpaths");
    let stderr = String::from_utf8(rcc_config_diag_command.output().await?.stderr)?;
    assert!(stderr.starts_with("OK.\n"));
    Ok(())
}

async fn assert_tasks() -> AnyhowResult<()> {
    let mut schtasks_cmd = Command::new("schtasks.exe");
    schtasks_cmd.arg("/query");
    let schtasks_output = schtasks_cmd.output().await?;
    assert!(schtasks_output.status.success());
    assert!(!String::from_utf8(schtasks_output.stdout)?.contains("robotmk"));
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
