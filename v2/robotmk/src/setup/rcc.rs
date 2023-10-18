use super::icacls::run_icacls_command;
use crate::config::internal::GlobalConfig;

use anyhow::{Context, Result};
use camino::Utf8Path;
use log::debug;

pub fn setup(global_config: &GlobalConfig) -> Result<()> {
    adjust_rcc_binary_permissions(&global_config.rcc_binary_path)
        .context("Failed to adjust permissions of RCC binary")
}

fn adjust_rcc_binary_permissions(executable_path: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` read and execute access to {executable_path}");
    run_icacls_command(vec![executable_path.as_str(), "/grant", "Users:(RX)"]).context(format!(
        "Adjusting permissions of {executable_path} for group `Users` failed",
    ))
}
