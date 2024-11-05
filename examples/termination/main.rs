mod process_tree;

use anyhow::Result as AnyhowResult;
use camino::Utf8PathBuf;
use clap::Parser;
use process_tree::check_tree_size;
use robotmk::config::RetryStrategy;
use robotmk::environment::{Environment, RCCEnvironment, SystemEnvironment};
use robotmk::plans::run_attempts_with_rebot;
use robotmk::rf::robot::Robot;
use robotmk::session::{CurrentSession, RunSpec, Session};
use std::env::var;
use std::thread;
use std::time::Duration;
use sysinfo::{get_current_pid, System};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
struct Args {
    /// Run with or without RCC
    #[arg(name = "RCC_BINARY_PATH")]
    pub rcc_binary_path: Option<Utf8PathBuf>,
}

fn main() -> AnyhowResult<()> {
    match Args::parse().rcc_binary_path {
        Some(rcc_binary_path) => rcc_main(rcc_binary_path),
        None => system_main(),
    }
}

fn system_main() -> AnyhowResult<()> {
    let mut system = System::new();
    let current_pid = get_current_pid().unwrap();
    let cargo_manifest_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let test_dir = Utf8PathBuf::from_path_buf(tempdir()?.into_path()).unwrap();
    let flag_file = test_dir.join("flag_file");
    let resource_file = test_dir.join("resource");
    let robot = Robot {
        robot_target: cargo_manifest_dir.join("examples/termination/tasks.robot"),
        n_attempts_max: 1,
        command_line_args: vec![
            "--variable".into(),
            format!("FLAG_FILE:{flag_file}"),
            "--variable".into(),
            format!("RESOURCE:{resource_file}"),
        ],
        retry_strategy: RetryStrategy::Complete,
    };
    let token = CancellationToken::new();
    let thread_token = token.clone();
    let running = thread::spawn(move || {
        run_attempts_with_rebot(
            &robot,
            "test",
            &Environment::System(SystemEnvironment {}),
            &Session::Current(CurrentSession {}),
            3,
            &thread_token,
            &test_dir,
        )
    });
    while !flag_file.exists() {
        // Wait for all children to be created
        thread::sleep(Duration::from_millis(250));
        if running.is_finished() {
            panic!("{:?}", running.join());
        }
    }
    assert_eq!(check_tree_size(&mut system, current_pid), 3);
    assert!(resource_file.exists());
    token.cancel();
    match running.join().unwrap() {
        Err(error) => {
            let message = format!("{error:?}");
            assert!(message.starts_with("Cancelled"), "Message: {message}")
        }
        ok => panic!("Cancellation failed: {ok:?}"),
    };
    assert_eq!(check_tree_size(&mut system, current_pid), 1);
    #[cfg(unix)]
    assert!(!resource_file.exists());
    Ok(())
}

fn rcc_main(rcc_binary_path: Utf8PathBuf) -> AnyhowResult<()> {
    let mut system = System::new();
    let current_pid = get_current_pid().unwrap();
    let cargo_manifest_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let test_dir = Utf8PathBuf::from_path_buf(tempdir()?.into_path()).unwrap();
    let flag_file = test_dir.join("flag_file");
    let resource_file = test_dir.join("resource");
    let robot = Robot {
        robot_target: cargo_manifest_dir.join("examples/termination/tasks.robot"),
        n_attempts_max: 1,
        command_line_args: vec![
            "--variable".into(),
            format!("FLAG_FILE:{flag_file}"),
            "--variable".into(),
            format!("RESOURCE:{resource_file}"),
        ],
        retry_strategy: RetryStrategy::Complete,
    };
    let rcc_environment = Environment::Rcc(RCCEnvironment {
        binary_path: rcc_binary_path,
        remote_origin: None,
        catalog_zip: None,
        robot_yaml_path: cargo_manifest_dir.join("examples/termination/robot.yaml"),
        controller: "termination_rcc".into(),
        space: "termination_rcc".into(),
        build_timeout: 1200,
        build_runtime_directory: test_dir.clone(),
        robocorp_home: test_dir.join("robocorp_home").to_string(),
    });
    let session = Session::Current(CurrentSession {});
    let build_instructions = rcc_environment.build_instructions().unwrap();
    let run_spec = RunSpec {
        id: "unused_id",
        command_spec: &build_instructions.build_command_spec,
        runtime_base_path: &build_instructions.runtime_directory,
        timeout: build_instructions.timeout,
        cancellation_token: &CancellationToken::new(),
    };
    session.run(&run_spec).unwrap();
    println!("Finished session build.");
    let token = CancellationToken::new();
    let thread_token = token.clone();
    let running = thread::spawn(move || {
        run_attempts_with_rebot(
            &robot,
            "test",
            &rcc_environment,
            &session,
            20,
            &thread_token,
            &test_dir,
        )
    });
    while !flag_file.exists() {
        // Wait for all children to be created
        thread::sleep(Duration::from_millis(250));
        if running.is_finished() {
            panic!("{:?}", running.join());
        }
    }
    assert_eq!(check_tree_size(&mut system, current_pid), 4);
    assert!(resource_file.exists());
    token.cancel();
    match running.join().unwrap() {
        Err(error) => {
            let message = format!("{error:?}");
            assert!(message.starts_with("Cancelled"), "Message: {message}")
        }
        ok => panic!("Cancellation failed: {ok:?}"),
    };
    assert_eq!(check_tree_size(&mut system, current_pid), 1);
    #[cfg(unix)]
    assert!(!resource_file.exists());
    Ok(())
}
