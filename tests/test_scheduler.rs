pub mod rcc;
use crate::rcc::read_configuration_diagnostics;
use anyhow::{bail, Result as AnyhowResult};
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(windows)]
use robotmk::config::UserSessionConfig;
use robotmk::config::{
    Config, CustomRCCProfileConfig, EnvironmentConfig, ExecutionConfig, PlanConfig, PlanMetadata,
    RCCConfig, RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig,
    SequentialPlanGroup, SessionConfig, Source, WorkingDirectoryCleanupConfig,
};
use robotmk::section::Host;
use serde_json::to_string;
use std::env::var;
#[cfg(windows)]
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, write};
use std::path::Path;
use std::time::Duration;
use tokio::{
    process::Command,
    select,
    time::{sleep, timeout},
};
use walkdir::WalkDir;

#[tokio::test]
#[ignore]
async fn test_scheduler() -> AnyhowResult<()> {
    let test_dir = Utf8PathBuf::from(var("TEST_DIR")?);
    let unconfigured_plan_working_dir = test_dir
        .join("working")
        .join("plans")
        .join("should_be_removed_during_scheduler_setup");
    let configured_plan_working_dir = test_dir.join("working").join("plans").join("rcc_headless");
    let configured_plan_previous_execution_dir =
        configured_plan_working_dir.join("should_still_exist_after_scheduler_run");
    create_dir_all(&test_dir)?;
    create_dir_all(&unconfigured_plan_working_dir)?;
    create_dir_all(&configured_plan_previous_execution_dir)?;
    #[cfg(windows)]
    let test_user = var("TEST_USER")?;
    #[cfg(windows)]
    {
        grant_full_access(&test_user, &configured_plan_working_dir).await?;
        assert_permissions(
            &configured_plan_working_dir,
            &format!("{test_user}:(OI)(CI)(F)"),
        )
        .await?;
    }

    #[cfg(windows)]
    let current_user_name = var("UserName")?;
    let config = create_config(
        &test_dir,
        &Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("minimal_suite"),
        &Utf8PathBuf::from(var("MANAGED_ROBOT_ARCHIVE_PATH")?),
        RCCConfig {
            binary_path: var("RCC_BINARY_PATH")?.into(),
            profile_config: RCCProfileConfig::Custom(create_custom_rcc_profile(&test_dir)?),
        },
        #[cfg(windows)]
        &current_user_name,
    );

    run_scheduler(
        &test_dir,
        &config,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await?;

    assert_working_directory(
        &config.working_directory,
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    assert!(!unconfigured_plan_working_dir.exists());
    assert!(configured_plan_previous_execution_dir.is_dir());
    #[cfg(windows)]
    assert!(!get_permissions(&configured_plan_working_dir)
        .await?
        .contains(&test_user));
    assert_results_directory(&config.results_directory);
    assert_managed_directory(
        &config.managed_directory,
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    assert_rcc(
        &config.rcc_config,
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    #[cfg(windows)]
    assert_tasks().await?;

    Ok(())
}

#[cfg(windows)]
async fn grant_full_access(user: &str, target_path: &Utf8Path) -> tokio::io::Result<()> {
    let mut icacls_command = Command::new("icacls.exe");
    icacls_command
        .arg(target_path)
        .args(["/grant", &format!("{user}:(OI)(CI)F"), "/T"]);
    assert!(icacls_command.output().await?.status.success());
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
    managed_robot_archive_path: &Utf8Path,
    rcc_config: RCCConfig,
    #[cfg(windows)] user_name_headed: &str,
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
                    #[cfg(windows)]
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
                        id: "managed_robot".into(),
                        source: Source::Managed {
                            tar_gz_path: managed_robot_archive_path.into(),
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
                        #[cfg(unix)]
                        session_config: SessionConfig::Current,
                        #[cfg(windows)]
                        session_config: SessionConfig::SpecificUser(UserSessionConfig {
                            user_name: user_name_headed.into(),
                        }),
                        working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                            120,
                        ),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "managed".into(),
                            suite_name: "robot_archive".into(),
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
    n_seconds_run_max: u64,
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

    select! {
        _ = await_plan_results(config) => {},
        _ = robotmk_child_proc.wait() => {
            bail!("Scheduler terminated unexpectedly")
        },
        _ = sleep(Duration::from_secs(n_seconds_run_max)) => {
            bail!(format!("No plan result files appeared with {n_seconds_run_max} seconds"))
        },
    };
    remove_file(&run_flag_path)?;
    assert!(timeout(Duration::from_secs(3), robotmk_child_proc.wait())
        .await
        .is_ok());

    Ok(())
}

async fn await_plan_results(config: &Config) {
    let expected_result_files: Vec<Utf8PathBuf> = config
        .plan_groups
        .iter()
        .flat_map(|plan_group| {
            plan_group.plans.iter().map(|plan_config| {
                config
                    .results_directory
                    .join("plans")
                    .join(format!("{}.json", &plan_config.id))
            })
        })
        .collect();
    loop {
        if expected_result_files
            .iter()
            .all(|expected_result_file| expected_result_file.is_file())
        {
            break;
        }
        sleep(Duration::from_secs(5)).await;
    }
}

async fn assert_working_directory(
    working_directory: &Utf8Path,
    #[cfg(windows)] headed_user_name: &str,
) -> AnyhowResult<()> {
    assert!(working_directory.is_dir());
    assert_eq!(
        directory_entries(working_directory, 1),
        ["environment_building", "plans", "rcc_setup"]
    );
    assert_eq!(
        directory_entries(working_directory.join("rcc_setup"), 2),
        [
            "current_user",
            "current_user/custom_profile_import.stderr",
            "current_user/custom_profile_import.stdout",
            "current_user/custom_profile_switch.stderr",
            "current_user/custom_profile_switch.stdout",
            "current_user/holotree_disabling_sharing.stderr",
            "current_user/holotree_disabling_sharing.stdout",
            #[cfg(windows)]
            "current_user/long_path_support_enabling.stderr",
            #[cfg(windows)]
            "current_user/long_path_support_enabling.stdout",
            "current_user/telemetry_disabling.stderr",
            "current_user/telemetry_disabling.stdout",
            #[cfg(windows)]
            &format!("user_{headed_user_name}"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_import.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_import.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_import.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_import.stdout"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_switch.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_switch.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_switch.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/custom_profile_switch.stdout"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/holotree_disabling_sharing.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/holotree_disabling_sharing.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/holotree_disabling_sharing.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/holotree_disabling_sharing.stdout"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/telemetry_disabling.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/telemetry_disabling.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/telemetry_disabling.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/telemetry_disabling.stdout"),
        ],
    );
    #[cfg(windows)]
    assert_permissions(
        working_directory
            .join("rcc_setup")
            .join(format!("user_{headed_user_name}")),
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    assert_eq!(
        directory_entries(working_directory.join("environment_building"), 2),
        [
            "current_user",
            #[cfg(unix)]
            "current_user/managed_robot.stderr",
            #[cfg(unix)]
            "current_user/managed_robot.stdout",
            "current_user/rcc_headless.stderr",
            "current_user/rcc_headless.stdout",
            #[cfg(windows)]
            &format!("user_{headed_user_name}"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/managed_robot.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/managed_robot.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/managed_robot.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/managed_robot.stdout"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/rcc_headed.bat"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/rcc_headed.exit_code"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/rcc_headed.stderr"),
            #[cfg(windows)]
            &format!("user_{headed_user_name}/rcc_headed.stdout"),
        ]
    );
    #[cfg(windows)]
    assert_permissions(
        working_directory
            .join("environment_building")
            .join(format!("user_{headed_user_name}")),
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    assert_eq!(
        directory_entries(working_directory.join("plans"), 1),
        [
            "managed_robot",
            "no_rcc",
            #[cfg(windows)]
            "rcc_headed",
            "rcc_headless"
        ]
    );

    // We expliclitly don't check for the rebot files in the case without RCC, since this must also
    // work on systems that don't have the necessary Python environment.
    assert!(!directory_entries(working_directory.join("plans").join("no_rcc"), 1).is_empty());

    #[cfg(windows)]
    {
        let entries_rcc_headed =
            directory_entries(working_directory.join("plans").join("rcc_headed"), 2).join("");
        assert!(entries_rcc_headed.contains("rebot.xml"));
        assert!(entries_rcc_headed.contains("1.bat"));
    }

    let entries_rcc_headless =
        directory_entries(working_directory.join("plans").join("rcc_headless"), 2).join("");
    assert!(entries_rcc_headless.contains("rebot.xml"));
    assert!(!entries_rcc_headless.contains("1.bat"));

    let entries_managed =
        directory_entries(working_directory.join("plans").join("managed_robot"), 2).join("");
    assert!(entries_managed.contains("rebot.xml"));

    Ok(())
}

#[cfg(windows)]
async fn get_permissions(path: impl AsRef<OsStr>) -> AnyhowResult<String> {
    let mut icacls_command = Command::new("icacls.exe");
    icacls_command.arg(path);
    let permissions = String::from_utf8(icacls_command.output().await?.stdout)?;
    Ok(permissions)
}

#[cfg(windows)]
async fn assert_permissions(path: impl AsRef<OsStr>, permissions: &str) -> AnyhowResult<()> {
    assert!(get_permissions(path).await?.contains(permissions));
    Ok(())
}

fn assert_results_directory(results_directory: &Utf8Path) {
    assert!(results_directory.is_dir());
    assert_eq!(
        directory_entries(results_directory, 2),
        [
            "environment_build_states.json",
            "plans",
            "plans/managed_robot.json",
            "plans/no_rcc.json",
            #[cfg(windows)]
            "plans/rcc_headed.json",
            "plans/rcc_headless.json",
            "scheduler_phase.json",
            "setup_failures.json"
        ]
    );
}

async fn assert_managed_directory(
    managed_directory: &Utf8Path,
    #[cfg(windows)] headed_user_name: &str,
) -> AnyhowResult<()> {
    assert!(managed_directory.is_dir());
    assert_eq!(directory_entries(managed_directory, 1), ["managed_robot"]);
    #[cfg(windows)]
    assert_permissions(
        &managed_directory.join("managed_robot"),
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    Ok(())
}

async fn assert_rcc(
    rcc_config: &RCCConfig,
    #[cfg(windows)] headed_user_name: &str,
) -> AnyhowResult<()> {
    #[cfg(windows)]
    assert_rcc_files_permissions(rcc_config, headed_user_name).await?;
    assert_rcc_configuration(rcc_config).await?;
    #[cfg(windows)]
    assert_rcc_longpath_support_enabled(&rcc_config.binary_path).await?;
    Ok(())
}

#[cfg(windows)]
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
    assert_eq!(
        diagnostics.details.get("holotree-shared").unwrap().as_str(),
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

#[cfg(windows)]
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

#[cfg(windows)]
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
        // align unix and windows
        .map(|s| s.replace("\\", "/"))
        .collect()
}
