use super::api::{self, SetupStep, StepWithPlans, skip};
use super::partition_into_conda_and_other_plans;

use crate::internal_config::{GlobalConfig, Plan};

use camino::Utf8PathBuf;

struct StepCopyBinary {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl SetupStep for StepCopyBinary {
    fn label(&self) -> String {
        format!(
            "Copy micromamba binary: `{source}` -> {target}",
            source = &self.source,
            target = &self.target,
        )
    }

    fn setup(&self) -> Result<(), api::Error> {
        std::fs::copy(&self.source, &self.target).map_err(|err| {
            api::Error::new(
                format!(
                    "Copying micromamba binary from `{}` to `{}` failed",
                    self.source, self.target
                ),
                err.into(),
            )
        })?;
        Ok(())
    }
}

pub fn gather_copy_micromamba_binary(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (conda_plans, other_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_conda_and_other_plans(plans);
    vec![
        skip(other_plans),
        (
            Box::new(StepCopyBinary {
                source: config.conda_config.original_micromamba_binary_path.clone(),
                target: config.conda_config.micromamba_binary_path(),
            }),
            conda_plans,
        ),
    ]
}
