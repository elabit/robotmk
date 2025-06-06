pub mod helper;
pub mod rcc;
use crate::helper::{await_plan_results, directory_entries, var};
use crate::rcc::read_configuration_diagnostics;
use anyhow::{Result as AnyhowResult, bail};
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(windows)]
use robotmk::config::UserSessionConfig;
use robotmk::config::{
    CondaConfig, CondaEnvironmentConfig, CondaEnvironmentSource, Config, CustomRCCProfileConfig,
    EnvironmentConfig, ExecutionConfig, HTTPProxyConfig, PlanConfig, PlanMetadata, RCCConfig,
    RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup,
    SessionConfig, Source, WorkingDirectoryCleanupConfig,
};
use robotmk::results::results_directory;
use robotmk::section::Host;
use serde_json::to_string;
#[cfg(windows)]
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, write};
use std::path::Path;
#[cfg(windows)]
use std::process::Output;
use std::time::Duration;
use tokio::{
    process::Command,
    select,
    time::{sleep, timeout},
};

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
    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path: Utf8PathBuf = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;
    let config = create_config(
        &test_dir,
        &Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("minimal_suite"),
        &Utf8PathBuf::from(var("MANAGED_ROBOT_ARCHIVE_PATH")?),
        RCCConfig {
            binary_path: var("RCC_BINARY_PATH")?.into(),
            profile_config: RCCProfileConfig::Custom(create_custom_rcc_profile(&test_dir)?),
            robocorp_home_base: temp_dir_path.join("rc_home_base"),
        },
        CondaConfig {
            micromamba_binary_path: var("MICROMAMBA_BINARY_PATH")?.into(),
            base_directory: temp_dir_path.join("conda_base"),
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
        &config.runtime_directory.join("working"),
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    assert!(!unconfigured_plan_working_dir.exists());
    assert!(configured_plan_previous_execution_dir.is_dir());
    #[cfg(windows)]
    assert!(
        !get_permissions(&configured_plan_working_dir)
            .await?
            .contains(&test_user)
    );
    assert_results_directory(&results_directory(&config.runtime_directory));
    assert_managed_directory(
        &config.runtime_directory.join("managed"),
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    assert_rcc(&config.rcc_config).await?;
    #[cfg(windows)]
    assert_tasks().await?;
    assert_sequentiality(
        config
            .runtime_directory
            .join("working")
            .join("plans")
            .join(&config.plan_groups[0].plans[0].id),
        config
            .runtime_directory
            .join("working")
            .join("plans")
            .join(&config.plan_groups[0].plans[1].id),
    );
    assert_robocorp_home(
        &config.rcc_config.robocorp_home_base,
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;
    assert_conda_base(
        &config.conda_config.base_directory,
        #[cfg(windows)]
        &current_user_name,
    )
    .await?;

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
    runtime_dir: &Utf8Path,
    suite_dir: &Utf8Path,
    managed_robot_archive_path: &Utf8Path,
    rcc_config: RCCConfig,
    conda_config: CondaConfig,
    #[cfg(windows)] user_name_headed: &str,
) -> Config {
    Config {
        runtime_directory: runtime_dir.into(),
        rcc_config,
        conda_config,
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
                            environment_variables_rendered_obfuscated: vec![],
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
                            catalog_zip: None,
                        }),
                        session_config: SessionConfig::Current,
                        working_directory_cleanup_config:
                            WorkingDirectoryCleanupConfig::MaxExecutions(4),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "app".into(),
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
                            environment_variables_rendered_obfuscated: vec![],
                        },
                        execution_config: ExecutionConfig {
                            n_attempts_max: 1,
                            retry_strategy: RetryStrategy::Complete,
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                            robot_yaml_path: "robot.yaml".into(),
                            build_timeout: 1200,
                            remote_origin: None,
                            catalog_zip: None,
                        }),
                        session_config: SessionConfig::SpecificUser(UserSessionConfig {
                            user_name: user_name_headed.into(),
                        }),
                        working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                            120,
                        ),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "app".into(),
                            suite_name: "minimal_suite".into(),
                            variant: "".into(),
                        },
                    },
                    PlanConfig {
                        id: "rcc_managed_robot".into(),
                        source: Source::Managed {
                            tar_gz_path: managed_robot_archive_path.into(),
                            version_number: 1,
                            version_label: "".into(),
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
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Rcc(RCCEnvironmentConfig {
                            robot_yaml_path: "robot.yaml".into(),
                            build_timeout: 1200,
                            remote_origin: None,
                            catalog_zip: None,
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
            SequentialPlanGroup {
                plans: vec![
                    PlanConfig {
                        id: "conda_headless".into(),
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
                            build_timeout: 1200,
                        }),
                        session_config: SessionConfig::Current,
                        working_directory_cleanup_config:
                            WorkingDirectoryCleanupConfig::MaxExecutions(4),
                        host: Host::Source,
                        metadata: PlanMetadata {
                            application: "app".into(),
                            suite_name: "minimal_suite".into(),
                            variant: "".into(),
                        },
                    },
                    #[cfg(windows)]
                    PlanConfig {
                        id: "conda_headed".into(),
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
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Conda(CondaEnvironmentConfig {
                            source: CondaEnvironmentSource::Manifest("conda.yaml".into()),
                            robotmk_manifest_path: None,
                            http_proxy_config: HTTPProxyConfig::default(),
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
                            application: "app".into(),
                            suite_name: "minimal_suite".into(),
                            variant: "".into(),
                        },
                    },
                    PlanConfig {
                        id: "conda_managed_robot".into(),
                        source: Source::Managed {
                            tar_gz_path: managed_robot_archive_path.into(),
                            version_number: 1,
                            version_label: "".into(),
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
                            timeout: 15,
                        },
                        environment_config: EnvironmentConfig::Conda(CondaEnvironmentConfig {
                            source: CondaEnvironmentSource::Manifest("conda.yaml".into()),
                            robotmk_manifest_path: None,
                            http_proxy_config: HTTPProxyConfig::default(),
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
                    id: "system_env".into(),
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
            #[cfg(windows)]
            "conda_headed",
            #[cfg(windows)]
            "conda_headed/create.stderr",
            #[cfg(windows)]
            "conda_headed/create.stdout",
            "conda_headless",
            "conda_headless/create.stderr",
            "conda_headless/create.stdout",
            "conda_managed_robot",
            "conda_managed_robot/create.stderr",
            "conda_managed_robot/create.stdout",
            #[cfg(windows)]
            "rcc_headed",
            #[cfg(windows)]
            "rcc_headed/build.bat",
            #[cfg(windows)]
            "rcc_headed/build.exit_code",
            #[cfg(windows)]
            "rcc_headed/build.stderr",
            #[cfg(windows)]
            "rcc_headed/build.stdout",
            "rcc_headless",
            "rcc_headless/build.stderr",
            "rcc_headless/build.stdout",
            "rcc_managed_robot",
            #[cfg(windows)]
            "rcc_managed_robot/build.bat",
            #[cfg(windows)]
            "rcc_managed_robot/build.exit_code",
            "rcc_managed_robot/build.stderr",
            "rcc_managed_robot/build.stdout",
        ]
    );
    #[cfg(windows)]
    assert_permissions(
        working_directory
            .join("environment_building")
            .join("rcc_headed"),
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    #[cfg(windows)]
    assert_permissions(
        working_directory
            .join("environment_building")
            .join("rcc_managed_robot"),
        &format!("{headed_user_name}:(OI)(CI)(F)"),
    )
    .await?;
    assert_eq!(
        directory_entries(working_directory.join("plans"), 1),
        [
            #[cfg(windows)]
            "conda_headed",
            "conda_headless",
            "conda_managed_robot",
            #[cfg(windows)]
            "rcc_headed",
            "rcc_headless",
            "rcc_managed_robot",
            "system_env",
        ]
    );

    // We expliclitly don't check for the rebot files in the case without RCC, since this must also
    // work on systems that don't have the necessary Python environment.
    assert!(!directory_entries(working_directory.join("plans").join("system_env"), 1).is_empty());

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

    let entries_rcc_managed =
        directory_entries(working_directory.join("plans").join("rcc_managed_robot"), 2).join("");
    assert!(entries_rcc_managed.contains("rebot.xml"));

    #[cfg(windows)]
    {
        let entries_conda_headed =
            directory_entries(working_directory.join("plans").join("conda_headed"), 2).join("");
        assert!(entries_conda_headed.contains("rebot.xml"));
        assert!(entries_conda_headed.contains("1.bat"));
    }

    let entries_conda_headless =
        directory_entries(working_directory.join("plans").join("conda_headless"), 2).join("");
    assert!(entries_conda_headless.contains("rebot.xml"));
    assert!(!entries_conda_headless.contains("1.bat"));

    let entries_conda_managed = directory_entries(
        working_directory.join("plans").join("conda_managed_robot"),
        2,
    )
    .join("");
    assert!(entries_conda_managed.contains("rebot.xml"));

    Ok(())
}

#[cfg(windows)]
async fn run_icacls<I, S>(args: I) -> std::io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut icacls_command = Command::new("icacls.exe");
    icacls_command.args(args);
    icacls_command.output().await
}

#[cfg(windows)]
async fn get_permissions(path: impl AsRef<OsStr>) -> AnyhowResult<String> {
    Ok(String::from_utf8(run_icacls(&[path]).await?.stdout)?)
}

#[cfg(windows)]
async fn assert_permissions(path: impl AsRef<OsStr>, permissions: &str) -> AnyhowResult<()> {
    assert!(get_permissions(path).await?.contains(permissions));
    Ok(())
}

#[cfg(windows)]
async fn dacl_exists_for_sid(path: &Utf8Path, sid: &str) -> AnyhowResult<bool> {
    Ok(run_icacls([path.as_str(), "/findsid", sid])
        .await?
        .status
        .success())
}

fn assert_results_directory(results_directory: &Utf8Path) {
    assert!(results_directory.is_dir());
    assert_eq!(
        directory_entries(results_directory, 2),
        [
            "environment_build_states.json",
            "plans",
            #[cfg(windows)]
            "plans/conda_headed.json",
            "plans/conda_headless.json",
            "plans/conda_managed_robot.json",
            #[cfg(windows)]
            "plans/rcc_headed.json",
            "plans/rcc_headless.json",
            "plans/rcc_managed_robot.json",
            "plans/system_env.json",
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
    assert_eq!(
        directory_entries(managed_directory, 1),
        ["conda_managed_robot", "rcc_managed_robot"]
    );
    #[cfg(windows)]
    {
        assert_permissions(
            &managed_directory.join("rcc_managed_robot"),
            &format!("{headed_user_name}:(OI)(CI)(F)"),
        )
        .await?;
        assert_permissions(
            &managed_directory.join("conda_managed_robot"),
            &format!("{headed_user_name}:(OI)(CI)(F)"),
        )
        .await?;
    }
    Ok(())
}

async fn assert_rcc(rcc_config: &RCCConfig) -> AnyhowResult<()> {
    #[cfg(windows)]
    assert_rcc_files_permissions(rcc_config).await?;
    assert_rcc_configuration(
        rcc_config,
        rcc_config
            .robocorp_home_base
            .join("current_user")
            .to_string()
            .as_str(),
    )
    .await?;
    Ok(())
}

#[cfg(windows)]
async fn assert_rcc_files_permissions(rcc_config: &RCCConfig) -> AnyhowResult<()> {
    assert!(dacl_exists_for_sid(&rcc_config.binary_path, "*S-1-5-32-545").await?);
    let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config else {
        return Ok(());
    };
    assert!(dacl_exists_for_sid(&custom_rcc_profile_config.path, "*S-1-5-32-545").await?);
    Ok(())
}

async fn assert_rcc_configuration(rcc_config: &RCCConfig, robocorp_home: &str) -> AnyhowResult<()> {
    let diagnostics = read_configuration_diagnostics(&rcc_config.binary_path, robocorp_home)?;
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
async fn assert_tasks() -> AnyhowResult<()> {
    let mut schtasks_cmd = Command::new("schtasks.exe");
    schtasks_cmd.arg("/query");
    let schtasks_output = schtasks_cmd.output().await?;
    assert!(schtasks_output.status.success());
    assert!(!String::from_utf8(schtasks_output.stdout)?.contains("robotmk"));
    Ok(())
}

fn assert_sequentiality(
    working_directory_first_plan: impl AsRef<Path>,
    working_directory_second_plan: impl AsRef<Path>,
) {
    let mut dir_entries_first_plan = directory_entries(working_directory_first_plan, 1);
    let mut dir_entries_second_plan = directory_entries(working_directory_second_plan, 1);
    dir_entries_first_plan.sort();
    dir_entries_second_plan.sort();
    assert!(dir_entries_first_plan[0] < dir_entries_second_plan[0]);
}

async fn assert_robocorp_home(
    robocorp_home_base: &Utf8Path,
    #[cfg(windows)] headed_user_name: &str,
) -> AnyhowResult<()> {
    assert!(robocorp_home_base.is_dir());
    assert_eq!(
        directory_entries(robocorp_home_base, 1),
        [
            "current_user",
            #[cfg(windows)]
            &format!("user_{headed_user_name}"),
        ]
    );
    #[cfg(windows)]
    {
        let permissions_robocorp_home_base = get_permissions(robocorp_home_base).await?;
        assert_eq!(
            permissions_robocorp_home_base
                .lines()
                .collect::<Vec<&str>>()
                .len(),
            4 // Administrator group + headed user name + empty line + success message (suppressing the latter with /q does not seem to work)
        );
        assert!(dacl_exists_for_sid(robocorp_home_base, "*S-1-5-32-544").await?);
        assert_permissions(&robocorp_home_base, &format!("{headed_user_name}:(R)")).await?;
        assert_permissions(
            &robocorp_home_base.join(format!("user_{headed_user_name}")),
            &format!("{headed_user_name}:(OI)(CI)(F)"),
        )
        .await?;
    }
    Ok(())
}

async fn assert_conda_base(
    conda_base: &Utf8Path,
    #[cfg(windows)] headed_user_name: &str,
) -> AnyhowResult<()> {
    assert!(conda_base.is_dir());
    assert_eq!(
        directory_entries(conda_base, 1),
        [
            "environments",
            "mamba_root_prefix",
            #[cfg(unix)]
            "micromamba",
            #[cfg(windows)]
            "micromamba.exe",
        ]
    );
    assert_eq!(
        directory_entries(conda_base.join("environments"), 1),
        [
            #[cfg(windows)]
            "conda_headed",
            "conda_headless",
            "conda_managed_robot",
        ]
    );
    #[cfg(windows)]
    {
        let permissions_conda_base = get_permissions(conda_base).await?;
        assert_eq!(
            permissions_conda_base.lines().collect::<Vec<&str>>().len(),
            4 // Administrator group + headed user name + empty line + success message (suppressing the latter with /q does not seem to work)
        );
        assert!(dacl_exists_for_sid(conda_base, "*S-1-5-32-544").await?);
        assert_permissions(&conda_base, &format!("{headed_user_name}:(OI)(CI)(RX)")).await?;
    }
    Ok(())
}
