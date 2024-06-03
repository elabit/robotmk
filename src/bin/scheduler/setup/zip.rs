use crate::internal_config::{Plan, Source};
use crate::setup::run_icacls_command;
use camino::Utf8Path;
use log::info;
use robotmk::lock::Locker;
use robotmk::results::ManagementFailues;
use robotmk::section::WriteSection;
use robotmk::session::Session;
use std::collections::HashMap;
use std::fs;
use std::io;

fn unzip_into(zip_file: &Utf8Path, target_path: &Utf8Path) -> anyhow::Result<()> {
    info!("Reading archive \"{}\"", zip_file);
    let file = fs::File::open(zip_file)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let target = target_path.join_os(outpath);
        if file.is_dir() {
            fs::create_dir_all(&target)?;
            info!("Directory created \"{}\"", target.display());
        } else {
            if let Some(p) = target.parent() {
                if !p.exists() {
                    info!("Directory created \"{}\"", p.display());
                    fs::create_dir_all(p)?;
                }
            }
            let mut target_file = fs::File::create(&target)?;
            io::copy(&mut file, &mut target_file)?;
            info!(
                "File extracted to \"{}\" ({} bytes)",
                target.display(),
                file.size()
            );
        }
    }
    Ok(())
}

fn zip_setup(plans: Vec<Plan>) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = Vec::new();
    let mut failures = HashMap::new();
    for plan in plans.into_iter() {
        if let Source::Managed { zip_file, target } = &plan.source {
            if let Err(error) = unzip_into(zip_file, target) {
                info!("{error:#}");
                failures.insert(plan.id.clone(), format!("{error:#}"));
                continue;
            }
            info!("Unzipped {} into `{}`.", zip_file, target);
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

fn grant_full_access(user: &str, target_path: &Utf8Path) -> anyhow::Result<()> {
    let arguments = [
        target_path.as_ref(),
        "/grant",
        &format!("{user}:(OI)(CI)F"),
        "/T",
    ];
    run_icacls_command(arguments).map_err(|e| {
        let message = format!("Adjusting permissions of {target_path} for user `{user}` failed");
        e.context(message)
    })
}

fn permission_setup(plans: Vec<Plan>) -> (Vec<Plan>, HashMap<String, String>) {
    let mut surviving_plans = Vec::new();
    let mut failures = HashMap::new();
    for plan in plans.into_iter() {
        if let Session::User(user_session) = &plan.session {
            if let Source::Managed { target, .. } = &plan.source {
                if let Err(error) = grant_full_access(&user_session.user_name, target) {
                    info!("{error:#}");
                    failures.insert(plan.id.clone(), format!("{error:#}"));
                    continue;
                }
                info!(
                    "Adjusted permissions for {} for user `{}`.",
                    target, &user_session.user_name
                );
            }
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
    let (surviving_plans, zip_failures) = zip_setup(plans);
    let (surviving_plans, permission_failures) = permission_setup(surviving_plans);
    ManagementFailues(
        zip_failures
            .into_iter()
            .chain(permission_failures.into_iter())
            .collect(),
    )
    .write(
        results_directory.join("management_failures.json"),
        results_directory_locker,
    )?;
    anyhow::Ok(surviving_plans)
}
