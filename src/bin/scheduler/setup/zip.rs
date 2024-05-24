use crate::internal_config::Plan;
use crate::setup::run_icacls_command;
use camino::Utf8Path;
use log::{error, info};
use robotmk::session::Session;
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

fn zip_setup(plans: Vec<Plan>) -> Vec<Plan> {
    let mut surviving_plans = Vec::new();
    for plan in plans.into_iter() {
        if let Some(zip_file) = &plan.zip_file {
            if let Err(error) = unzip_into(zip_file, &plan.managed_directory) {
                error!("{error:#}");
                continue;
            }
            info!("Unzipped {} into `{}`.", zip_file, &plan.managed_directory);
        }
        surviving_plans.push(plan);
    }
    surviving_plans
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

fn permission_setup(plans: Vec<Plan>) -> Vec<Plan> {
    let mut surviving_plans = Vec::new();
    for plan in plans.into_iter() {
        if let Session::User(user_session) = &plan.session {
            if let Err(error) = grant_full_access(&user_session.user_name, &plan.managed_directory)
            {
                error!("{error:#}");
                continue;
            }
            info!(
                "Adjusted permissions for {} for user `{}`.",
                &plan.managed_directory, &user_session.user_name
            );
        }
        surviving_plans.push(plan);
    }
    surviving_plans
}

pub fn setup(plans: Vec<Plan>) -> Vec<Plan> {
    permission_setup(zip_setup(plans))
}
