use anyhow::Result;
use camino::Utf8PathBuf;
use robotmk::config::RetryStrategy;
use robotmk::environment::{Environment, SystemEnvironment};
use robotmk::rf::robot::Robot;
use robotmk::sessions::session::{CurrentSession, Session};
use robotmk::suites::run_attempts_with_rebot;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[test]
fn test_rebot_run() -> Result<()> {
    let test_dir = Utf8PathBuf::from_path_buf(tempdir()?.into_path()).unwrap();
    let robot = Robot {
        robot_target: "tests/minimal_suite/tasks.robot".into(),
        n_attempts_max: 1,
        command_line_args: vec![],
        retry_strategy: RetryStrategy::Complete,
    };
    let (attempt_outcomes, rebot) = run_attempts_with_rebot(
        &robot,
        "test",
        &Environment::System(SystemEnvironment {}),
        &Session::Current(CurrentSession {}),
        3,
        &CancellationToken::default(),
        &test_dir,
    )?;
    assert_eq!(attempt_outcomes, &[AttemptOutcome::AllTestsPassed]);
    assert!(rebot.is_some());
    Ok(())
}
