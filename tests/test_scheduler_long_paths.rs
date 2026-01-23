pub mod helper;
pub mod rcc;
use crate::helper::{run_scheduler, var};
use anyhow::Result as AnyhowResult;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{
    Config, EnvironmentConfig, ExecutionConfig, PlanConfig, PlanMetadata, RCCConfig,
    RCCEnvironmentConfig, RCCProfileConfig, RetryStrategy, RobotConfig, SequentialPlanGroup,
    SessionConfig, Source, WorkingDirectoryCleanupConfig,
};
use robotmk::section::Host;

// Test that the scheduler can handle very long paths in the runtime directory on Windows.
// The scheduler setup involves permission changes and ownership transfers, which are prone
// to fail on long paths if not handled correctly.
#[tokio::test]
#[ignore]
async fn test_scheduler_handles_long_paths() -> AnyhowResult<()> {
    let test_dir = tempfile::tempdir()?;
    let test_dir_path = Utf8PathBuf::try_from(test_dir.path().to_path_buf())?;
    let mut very_long_sub_dir = test_dir_path.clone();
    for _ in 0..15 {
        very_long_sub_dir = very_long_sub_dir.join("very___long___subdir");
    }
    std::fs::create_dir_all(&very_long_sub_dir)?;
    std::fs::write(very_long_sub_dir.join("some_file"), "")?;

    let config: Config = create_config(
        &test_dir_path,
        &Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?)
            .join("tests")
            .join("minimal_suite"),
        RCCConfig {
            binary_path: var("RCC_BINARY_PATH")?.into(),
            profile_config: RCCProfileConfig::Default,
            robocorp_home_base: test_dir_path.join("rc_home_base"),
        },
    );

    run_scheduler(
        &test_dir_path,
        &config,
        var("N_SECONDS_RUN_MAX")?.parse::<u64>()?,
    )
    .await
}

fn create_config(runtime_dir: &Utf8Path, suite_dir: &Utf8Path, rcc_config: RCCConfig) -> Config {
    Config {
        runtime_directory: runtime_dir.into(),
        rcc_config,
        plan_groups: vec![SequentialPlanGroup {
            plans: vec![PlanConfig {
                id: "rcc_plan".into(),
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
