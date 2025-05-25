#![cfg(windows)]

use super::api::{self, SetupStep, StepWithPlans};
use crate::internal_config::{GlobalConfig, Plan};
use anyhow::Context;
use tempfile::tempdir;
use windows_registry::{LOCAL_MACHINE, Transaction};

pub fn gather_long_path_support(
    _global_config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    vec![(Box::new(StepLongPathSupport {}), plans)]
}

struct StepLongPathSupport {}

impl SetupStep for StepLongPathSupport {
    fn label(&self) -> String {
        "Enable long path support".into()
    }

    fn setup(&self) -> Result<(), api::Error> {
        Self::enable_long_path_support()?;
        Self::test_long_path_support()?;
        Ok(())
    }
}

impl StepLongPathSupport {
    fn enable_long_path_support() -> Result<(), api::Error> {
        let tx = Transaction::new().map_err(|e| {
            api::Error::new(
                "Failed to create registry transaction for enabling long path support".into(),
                e.into(),
            )
        })?;
        let key = LOCAL_MACHINE
            .options()
            .create()
            .write()
            .transaction(&tx)
            .open("SYSTEM\\CurrentControlSet\\Control\\FileSystem")
            .map_err(|e| {
                api::Error::new(
                    "Failed to open HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\FileSystem for enabling long path support".into(),
                    e.into(),
                )
            })?;
        key.set_u32("LongPathsEnabled", 1).map_err(|e| {
            api::Error::new(
                "Failed to set LongPathsEnabled to 1 for enabling long path support".into(),
                e.into(),
            )
        })?;

        tx.commit().map_err(|e| {
            api::Error::new(
                "Failed to commit registry transaction for enabling long path support".into(),
                e.into(),
            )
        })?;

        Ok(())
    }

    fn test_long_path_support() -> Result<(), api::Error> {
        let temp_dir = tempdir().map_err(|e| {
            api::Error::new(
                "Failed to create a temporary directory for verifying long path support".into(),
                e.into(),
            )
        })?;
        let mut very_long_path = temp_dir.path().to_path_buf();
        for _ in 0..15 {
            very_long_path = very_long_path.join("very___long___subdir");
        }

        std::fs::create_dir_all(&very_long_path).context(format!("Failed to create directory {}", very_long_path.display()))
            .map_err(|e| {
                api::Error::new(
                    "Failed to create a directory with a very long path to verify long path support".into(),
                    e,
                )}
            )?;

        Ok(())
    }
}
