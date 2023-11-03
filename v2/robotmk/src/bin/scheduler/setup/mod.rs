mod general;
mod icacls;
mod rcc;

use crate::config::internal::{GlobalConfig, Suite};

use anyhow::{Context, Result};
use log::debug;

pub fn setup(global_config: &GlobalConfig, suites: Vec<Suite>) -> Result<Vec<Suite>> {
    general::setup(global_config, &suites).context("General setup failed")?;
    debug!("General setup completed");
    let suites = rcc::setup(global_config, suites).context("RCC-specific setup failed")?;
    debug!("RCC-specific setup completed");
    Ok(suites)
}
