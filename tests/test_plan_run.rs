use anyhow::Result as AnyhowResult;
use camino::Utf8PathBuf;
use robotmk::config::RetryStrategy;
use robotmk::environment::{Environment, SystemEnvironment};
use robotmk::plans::run_attempts_with_rebot;
use robotmk::results::AttemptOutcome;
use robotmk::rf::robot::Robot;
use robotmk::session::{CurrentSession, Session};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[test]
#[ignore]
fn test_rebot_run() -> AnyhowResult<()> {
    let test_dir = Utf8PathBuf::from_path_buf(tempdir()?.into_path()).unwrap();
    let robot = Robot {
        robot_target: "tests/minimal_suite/tasks.robot".into(),
        n_attempts_max: 1,
        command_line_args: vec![],
        envs_rendered_obfuscated: vec![],
        retry_strategy: RetryStrategy::Complete,
    };
    let (attempt_reports, rebot) = run_attempts_with_rebot(
        &robot,
        "test",
        &Environment::System(SystemEnvironment {}),
        &Session::Current(CurrentSession {}),
        3,
        &CancellationToken::default(),
        &test_dir,
    )?;
    assert_eq!(attempt_reports.len(), 1);
    let attempt_report = &attempt_reports[0];
    assert_eq!(attempt_report.index, 1);
    assert_eq!(attempt_report.outcome, AttemptOutcome::AllTestsPassed);
    assert!(rebot.is_some());
    Ok(())
}

#[test]
#[ignore]
fn test_timeout_process() -> AnyhowResult<()> {
    let test_dir = Utf8PathBuf::from_path_buf(tempdir()?.into_path()).unwrap();
    let resource = test_dir.join("resource");
    let robot = Robot {
        robot_target: "tests/timeout/tasks.robot".into(),
        n_attempts_max: 1,
        command_line_args: vec!["--variable".into(), format!("RESOURCE:{resource}")],
        envs_rendered_obfuscated: vec![],
        retry_strategy: RetryStrategy::Complete,
    };
    let (attempt_reports, rebot) = run_attempts_with_rebot(
        &robot,
        "test",
        &Environment::System(SystemEnvironment {}),
        &Session::Current(CurrentSession {}),
        1,
        &CancellationToken::default(),
        &test_dir,
    )?;
    assert_eq!(attempt_reports.len(), 1);
    let attempt_report = &attempt_reports[0];
    assert_eq!(attempt_report.index, 1);
    assert_eq!(attempt_report.outcome, AttemptOutcome::TimedOut);
    assert!(rebot.is_none());
    #[cfg(unix)]
    assert!(!resource.is_file());
    Ok(())
}
