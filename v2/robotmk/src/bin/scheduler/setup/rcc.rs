use super::icacls::run_icacls_command;
use crate::command_spec::CommandSpec;
use crate::config::internal::{sort_suites_by_name, GlobalConfig, Suite};
use crate::environment::Environment;
use crate::results::RCCSetupFailures;
use crate::sessions::session::{CurrentSession, RunOutcome, RunSpec, Session};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use std::collections::HashMap;
use std::fs::create_dir_all;

pub fn setup(global_config: &GlobalConfig, suites: Vec<Suite>) -> Result<Vec<Suite>> {
    adjust_rcc_binary_permissions(&global_config.rcc_binary_path)
        .context("Failed to adjust permissions of RCC binary")?;
    create_dir_all(rcc_setup_working_directory(
        &global_config.working_directory,
    ))
    .context("Failed to create working directory for RCC setup")?;

    let (rcc_suites, mut surviving_suites): (Vec<Suite>, Vec<Suite>) = suites
        .into_iter()
        .partition(|suite| matches!(suite.environment, Environment::Rcc(_)));
    surviving_suites.append(&mut rcc_setup(global_config, rcc_suites)?);
    sort_suites_by_name(&mut surviving_suites);
    Ok(surviving_suites)
}

fn adjust_rcc_binary_permissions(executable_path: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` read and execute access to {executable_path}");
    run_icacls_command(vec![executable_path.as_str(), "/grant", "Users:(RX)"]).context(format!(
        "Adjusting permissions of {executable_path} for group `Users` failed",
    ))
}

fn rcc_setup_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("rcc_setup")
}

fn rcc_setup(global_config: &GlobalConfig, rcc_suites: Vec<Suite>) -> Result<Vec<Suite>> {
    let mut rcc_setup_failures = RCCSetupFailures {
        telemetry_disabling: vec![],
        shared_holotree: vec![],
        holotree_init: vec![],
    };

    debug!("Disabling RCC telemetry");
    let (sucessful_suites, failed_suites) = disable_rcc_telemetry(global_config, rcc_suites)
        .context("Disabling RCC telemetry failed")?;
    rcc_setup_failures.telemetry_disabling =
        failed_suites.into_iter().map(|suite| suite.name).collect();
    if !rcc_setup_failures.telemetry_disabling.is_empty() {
        error!(
            "Dropping the following suites due RCC telemetry disabling failure: {}",
            rcc_setup_failures.telemetry_disabling.join(", ")
        );
    }

    debug!("Initializing shared holotree");
    let (sucessful_suites, failed_suites) = shared_holotree_init(global_config, sucessful_suites)
        .context("Shared holotree initialization failed")?;
    rcc_setup_failures.shared_holotree =
        failed_suites.into_iter().map(|suite| suite.name).collect();
    if !rcc_setup_failures.shared_holotree.is_empty() {
        error!(
            "Dropping the following suites due to shared holotree initialization failure: {}",
            rcc_setup_failures.shared_holotree.join(", ")
        );
    }

    debug!("Initializing holotree");
    let (sucessful_suites, failed_suites) =
        holotree_init(global_config, sucessful_suites).context("Holotree initialization failed")?;
    rcc_setup_failures.holotree_init = failed_suites.into_iter().map(|suite| suite.name).collect();
    if !rcc_setup_failures.holotree_init.is_empty() {
        error!(
            "Dropping the following suites due to holotree initialization failure: {}",
            rcc_setup_failures.holotree_init.join(", ")
        );
    }

    rcc_setup_failures.write_atomic(
        &global_config.working_directory,
        &global_config.results_directory,
    )?;

    Ok(sucessful_suites)
}

fn disable_rcc_telemetry(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_binary_path.to_string(),
            arguments: vec![
                "configure".into(),
                "identity".into(),
                "--do-not-track".into(),
            ],
        },
        "rcc_telemetry_disabling",
    )
}

fn shared_holotree_init(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    Ok(
        if run_command_spec_in_session(
            &Session::Current(CurrentSession {}),
            &RunSpec {
                id: "rcc_shared_holotree_init",
                command_spec: &CommandSpec {
                    executable: global_config.rcc_binary_path.to_string(),
                    arguments: vec![
                        "holotree".into(),
                        "shared".into(),
                        "--enable".into(),
                        "--once".into(),
                    ],
                },
                base_path: &rcc_setup_working_directory(&global_config.working_directory)
                    .join("shared_holotree_init"),
                timeout: 120,
                termination_flag: &global_config.termination_flag,
            },
        )? {
            (suites, vec![])
        } else {
            error!(
            "Shared holotree initialization failed for the following suites which will now be dropped: {}",
            suites
                .iter()
                .map(|suite| suite.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );
            (vec![], suites)
        },
    )
}

fn holotree_init(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_binary_path.to_string(),
            arguments: vec!["holotree".into(), "init".into()],
        },
        "holotree_initialization",
    )
}

fn run_command_spec_per_session(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
    command_spec: &CommandSpec,
    id: &str,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    let mut suites_by_session = HashMap::new();
    for suite in suites {
        suites_by_session
            .entry(suite.session.clone())
            .or_insert(vec![])
            .push(suite);
    }
    let mut succesful_suites = vec![];
    let mut failed_suites = vec![];

    for (session, suites) in suites_by_session {
        let session_id = format!(
            "{}_{}",
            id,
            match &session {
                Session::Current(_) => "current_user".into(),
                Session::User(user_session) => format!("user_{}", user_session.user_name),
            }
        );

        debug!("Running {} for `{}`", command_spec, &session);
        if run_command_spec_in_session(
            &session,
            &RunSpec {
                id: &format!("robotmk_{session_id}"),
                command_spec,
                base_path: &rcc_setup_working_directory(&global_config.working_directory)
                    .join(session_id),
                timeout: 120,
                termination_flag: &global_config.termination_flag,
            },
        )? {
            succesful_suites.extend(suites);
        } else {
            failed_suites.extend(suites);
        }
    }

    Ok((succesful_suites, failed_suites))
}

fn run_command_spec_in_session(session: &Session, run_spec: &RunSpec) -> Result<bool> {
    match session.run(run_spec).context(format!(
        "Failed to run {} for `{session}`",
        run_spec.command_spec
    ))? {
        RunOutcome::Exited(exit_code) => match exit_code {
            Some(exit_code) => {
                if exit_code == 0 {
                    debug!("{} for `{session}` successful", run_spec.command_spec);
                    Ok(true)
                } else {
                    error!(
                        "{} for `{session}` exited non-successfully",
                        run_spec.command_spec
                    );
                    Ok(false)
                }
            }
            None => {
                error!(
                    "Failed to query exit code of {} for `{session}`",
                    run_spec.command_spec
                );
                Ok(false)
            }
        },
        RunOutcome::TimedOut => {
            error!("{} for `{session}` timed out", run_spec.command_spec);
            Ok(false)
        }
        RunOutcome::Terminated => bail!("Terminated"),
    }
}
