use crate::internal_config::{Plan, Source};
use camino::Utf8Path;
use flate2::read::GzDecoder;
use log::info;
use robotmk::lock::Locker;
use robotmk::results::ManagementFailues;
use robotmk::section::WriteSection;
use std::collections::HashMap;
use std::fs::File;
use tar::Archive;

fn unpack_into(tar_gz_path: &Utf8Path, target_path: &Utf8Path) -> anyhow::Result<()> {
    info!("Extracting archive \"{tar_gz_path}\"");
    let tar_gz = File::open(tar_gz_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target_path)?;
    Ok(())
}

fn unpack_setup(plans: Vec<Plan>) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = Vec::new();
    let mut failures = HashMap::new();
    for plan in plans.into_iter() {
        if let Source::Managed {
            tar_gz_path,
            target,
        } = &plan.source
        {
            if let Err(error) = unpack_into(tar_gz_path, target) {
                info!("{error:#}");
                failures.insert(plan.id.clone(), format!("{error:#}"));
                continue;
            }
            info!("Unpacked {} into `{}`.", tar_gz_path, target);
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

pub fn setup(
    results_directory: &Utf8Path,
    results_directory_locker: &Locker,
    plans: Vec<Plan>,
) -> anyhow::Result<Vec<Plan>> {
    let (surviving_plans, unpack_failures) = unpack_setup(plans);
    ManagementFailues(unpack_failures).write(
        results_directory.join("management_failures.json"),
        results_directory_locker,
    )?;
    anyhow::Ok(surviving_plans)
}
