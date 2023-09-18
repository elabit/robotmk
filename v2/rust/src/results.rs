use std::path::{Path, PathBuf};

pub fn suite_results_directory(results_directory: &Path) -> PathBuf {
    results_directory.join("suites")
}

pub fn suite_result_file(suite_results_dir: &Path, suite_name: &str) -> PathBuf {
    suite_results_dir.join(format!("{}.json", suite_name))
}
