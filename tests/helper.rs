use anyhow::Context;
use anyhow::{bail, Result as AnyhowResult};
use assert_cmd::cargo::cargo_bin;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::Config;
use robotmk::results::{plan_results_directory, results_directory};
use serde_json::to_string;
use std::ffi::OsStr;
use std::fs::{remove_file, write};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tokio::{process::Command, select, time::timeout};
use walkdir::WalkDir;

pub fn var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    std::env::var(key.as_ref()).context(format!(
        "Could not read: {}",
        key.as_ref().to_string_lossy()
    ))
}

pub async fn run_scheduler(
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
    assert!(timeout(Duration::from_secs(3), robotmk_child_proc.wait())
        .await
        .is_ok());

    Ok(())
}

pub async fn await_plan_results(config: &Config) {
    let expected_result_files: Vec<Utf8PathBuf> = config
        .plan_groups
        .iter()
        .flat_map(|plan_group| {
            plan_group.plans.iter().map(|plan_config| {
                plan_results_directory(&results_directory(&config.runtime_directory))
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

pub fn directory_entries(directory: impl AsRef<Path>, max_depth: usize) -> Vec<String> {
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
