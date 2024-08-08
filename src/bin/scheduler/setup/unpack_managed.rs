use crate::internal_config::{Plan, Source};
use camino::Utf8Path;
use flate2::read::GzDecoder;
use log::{error, info};
use robotmk::results::SetupFailure;
use std::fs::File;
use tar::Archive;

pub fn setup(plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = Vec::new();
    let mut failures = vec![];
    for plan in plans.into_iter() {
        if let Source::Managed {
            tar_gz_path,
            target,
        } = &plan.source
        {
            if let Err(error) = unpack_into(tar_gz_path, target) {
                error!(
                    "Plan {}: Failed to unpack managed source archive. Plan won't be scheduled.
                     Error: {error:?}",
                    plan.id
                );
                failures.push(SetupFailure {
                    plan_id: plan.id.clone(),
                    summary: "Failed to unpack managed source archive".to_string(),
                    details: format!("{error:?}"),
                });
                continue;
            }
            info!("Unpacked {} into `{}`.", tar_gz_path, target);
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

fn unpack_into(tar_gz_path: &Utf8Path, target_path: &Utf8Path) -> anyhow::Result<()> {
    info!("Extracting archive \"{tar_gz_path}\"");
    open_tar_gz_archive(tar_gz_path)?.unpack(target_path)?;
    Ok(())
}

fn open_tar_gz_archive(path: &Utf8Path) -> std::io::Result<Archive<GzDecoder<File>>> {
    let tar_gz = File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    Ok(Archive::new(tar))
}
