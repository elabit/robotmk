use anyhow::Context;
use camino::Utf8Path;
use std::fs;

pub fn create_dir_all<P: AsRef<Utf8Path>>(path: P) -> anyhow::Result<()> {
    fs::create_dir_all(path.as_ref()).context(format!("Failed to create dir `{}`", path.as_ref()))
}

pub fn remove_dir_all<P: AsRef<Utf8Path>>(path: P) -> anyhow::Result<()> {
    fs::remove_dir_all(path.as_ref()).context(format!("Failed to remove dir `{}`", path.as_ref()))
}

pub fn remove_file<P: AsRef<Utf8Path>>(path: P) -> anyhow::Result<()> {
    fs::remove_file(path.as_ref()).context(format!("Failed to remove file `{}`", path.as_ref()))
}
