mod process_tree;

use anyhow::Result as AnyhowResult;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use clap::{Parser, Subcommand};
use process_tree::check_tree_size;
use robotmk::config::{CondaEnvironmentSource, HTTPProxyConfig, RetryStrategy};
use robotmk::env::{
    Environment, conda::CondaEnvironment, rcc::RCCEnvironment, system::SystemEnvironment,
};
use robotmk::plans::run_attempts_with_rebot;
use robotmk::results::BuildOutcome;
use robotmk::rf::robot::Robot;
use robotmk::session::{CurrentSession, Session};
use std::env::var;
use std::thread;
use std::time::Duration;
use sysinfo::{System, get_current_pid};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    SystemPython,
    Rcc(NonSystemArgs),
    Micromamba(NonSystemArgs),
}

#[derive(Parser)]
struct NonSystemArgs {
    #[arg(name = "BINARY_PATH")]
    pub binary_path: Utf8PathBuf,
}

fn main() -> AnyhowResult<()> {
    match Args::parse().mode {
        Mode::SystemPython => system_main(),
        Mode::Rcc(args) => rcc_main(args.binary_path),
        Mode::Micromamba(args) => micromamba_main(args.binary_path),
    }
}

fn system_main() -> AnyhowResult<()> {
    let mut system = System::new();
    let current_pid = get_current_pid().unwrap();
    let cargo_manifest_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let test_dir = tempdir()?;
    let test_dir_path = Utf8Path::from_path(test_dir.path()).unwrap().to_path_buf();
    let flag_file = test_dir_path.join("flag_file");
    let resource_file = test_dir_path.join("resource");
    let robot = Robot {
        robot_target: cargo_manifest_dir.join("examples/termination/tasks.robot"),
        n_attempts_max: 1,
        command_line_args: vec![
            "--variable".into(),
            format!("FLAG_FILE:{flag_file}"),
            "--variable".into(),
            format!("RESOURCE:{resource_file}"),
        ],
        envs_rendered_obfuscated: vec![],
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
            &test_dir_path,
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
    // On Windows, we always kill the process tree, so no teardown will run.
    #[cfg(unix)]
    assert!(!resource_file.exists());
    Ok(())
}

fn rcc_main(rcc_binary_path: Utf8PathBuf) -> AnyhowResult<()> {
    let mut system = System::new();
    let current_pid = get_current_pid().unwrap();
    let cargo_manifest_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let test_dir = tempdir()?;
    let test_dir_path = Utf8Path::from_path(test_dir.path()).unwrap().to_path_buf();
    let flag_file = test_dir_path.join("flag_file");
    let resource_file = test_dir_path.join("resource");
    let robot = Robot {
        robot_target: cargo_manifest_dir.join("examples/termination/tasks.robot"),
        n_attempts_max: 1,
        command_line_args: vec![
            "--variable".into(),
            format!("FLAG_FILE:{flag_file}"),
            "--variable".into(),
            format!("RESOURCE:{resource_file}"),
        ],
        envs_rendered_obfuscated: vec![],
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
        build_runtime_directory: test_dir_path.to_path_buf(),
        robocorp_home: test_dir_path.join("robocorp_home").to_string(),
    });
    let session = Session::Current(CurrentSession {});
    assert!(matches!(
        rcc_environment
            .build("unused_id", &session, Utc::now(), &CancellationToken::new())
            .unwrap(),
        BuildOutcome::Success(_),
    ));
    println!("Finished environment build.");
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
            &test_dir_path,
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
    // On Windows, we always kill the process tree, so no teardown will run.
    #[cfg(unix)]
    assert!(!resource_file.exists());
    Ok(())
}

fn micromamba_main(micromamba_binary_path: Utf8PathBuf) -> AnyhowResult<()> {
    let mut system = System::new();
    let current_pid = get_current_pid().unwrap();
    let cargo_manifest_dir = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let test_dir = tempdir()?;
    let test_dir_path = Utf8Path::from_path(test_dir.path()).unwrap().to_path_buf();
    let flag_file = test_dir_path.join("flag_file");
    let resource_file = test_dir_path.join("resource");
    let robot = Robot {
        robot_target: cargo_manifest_dir.join("examples/termination/tasks.robot"),
        n_attempts_max: 1,
        command_line_args: vec![
            "--variable".into(),
            format!("FLAG_FILE:{flag_file}"),
            "--variable".into(),
            format!("RESOURCE:{resource_file}"),
        ],
        envs_rendered_obfuscated: vec![],
        retry_strategy: RetryStrategy::Complete,
    };
    let conda_environment = Environment::Conda(CondaEnvironment {
        source: CondaEnvironmentSource::Manifest(
            cargo_manifest_dir.join("examples/termination/conda.yaml"),
        ),
        robotmk_manifest_path: None,
        micromamba_binary_path,
        root_prefix: test_dir_path.join("micromamba_root"),
        prefix: test_dir_path.join("conda_env"),
        http_proxy_config: HTTPProxyConfig::default(),
        build_timeout: 1200,
        build_runtime_directory: test_dir_path.clone(),
    });
    let session = Session::Current(CurrentSession {});
    assert!(matches!(
        conda_environment
            .build("unused_id", &session, Utc::now(), &CancellationToken::new())
            .unwrap(),
        BuildOutcome::Success(_),
    ));
    println!("Finished environment build.");
    let token = CancellationToken::new();
    let thread_token = token.clone();
    let running = thread::spawn(move || {
        run_attempts_with_rebot(
            &robot,
            "test",
            &conda_environment,
            &session,
            20,
            &thread_token,
            &test_dir_path,
        )
    });
    while !flag_file.exists() {
        // Wait for all children to be created
        thread::sleep(Duration::from_millis(250));
        if running.is_finished() {
            panic!("{:?}", running.join());
        }
    }
    #[cfg(unix)]
    // 203317 Run "termination" Some("/home/joergherbel/git/robotmk/target/x86_64-unknown-linux-gnu/debug/examples/termination")
    // -205082 Sleep "micromamba" Some("/home/joergherbel/tmp/micromamba_fun/micromamba")
    // --205085 Sleep "python" Some("/tmp/.tmpgCIOOH/conda_env/bin/python3.12")
    // ---205171 Sleep "python" Some("/tmp/.tmpgCIOOH/conda_env/bin/python3.12")
    assert_eq!(check_tree_size(&mut system, current_pid), 4);
    #[cfg(windows)]
    // 9856 Run "termination.exe" Some("\\\\VBoxSvr\\robotmk\\target\\x86_64-pc-windows-msvc\\debug\\examples\\termination.exe")
    // -6500 Run "micromamba.exe" Some("C:\\Users\\vagrant\\Downloads\\micromamba.exe")
    // --9396 Run "cmd.exe" Some("C:\\Windows\\System32\\cmd.exe")
    // ---3236 Run "python.exe" Some("C:\\Users\\vagrant\\AppData\\Local\\Temp\\.tmpVnsIpp\\conda_env\\python.exe")
    // ----5032 Run "python.exe" Some("C:\\Users\\vagrant\\AppData\\Local\\Temp\\.tmpVnsIpp\\conda_env\\python.exe")
    assert_eq!(check_tree_size(&mut system, current_pid), 5);
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
    // On Windows, we always kill the process tree, so no teardown will run.
    #[cfg(unix)]
    assert!(!resource_file.exists());
    Ok(())
}
