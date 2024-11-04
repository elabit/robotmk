use camino::Utf8PathBuf;
use robotmk::config::Config;
use robotmk::results::{plan_results_directory, results_directory};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use walkdir::WalkDir;

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
