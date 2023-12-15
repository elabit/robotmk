use super::icacls::run_icacls_command;
use crate::internal_config::{sort_suites_by_id, GlobalConfig, Suite};
use crate::logging::log_and_return_error;
use robotmk::command_spec::CommandSpec;
use robotmk::environment::Environment;
use robotmk::results::RCCSetupFailures;
use robotmk::sessions::session::{CurrentSession, RunOutcome, RunSpec, Session};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use robotmk::{
    config::{CustomRCCProfileConfig, RCCProfileConfig},
    section::WriteSection,
};
use std::collections::HashMap;
use std::fs::{create_dir_all, remove_dir_all};
use std::vec;

pub fn setup(global_config: &GlobalConfig, suites: Vec<Suite>) -> Result<Vec<Suite>> {
    adjust_rcc_binary_permissions(&global_config.rcc_config.binary_path)
        .context("Failed to adjust permissions of RCC binary")?;
    clear_rcc_setup_working_directory(&rcc_setup_working_directory(
        &global_config.working_directory,
    ))?;
    if let RCCProfileConfig::Custom(custom_rcc_profile_config) =
        &global_config.rcc_config.profile_config
    {
        adjust_rcc_profile_permissions(&custom_rcc_profile_config.path)
            .context("Failed to adjust permissions of RCC profile")?;
    }

    let (rcc_suites, mut surviving_suites): (Vec<Suite>, Vec<Suite>) = suites
        .into_iter()
        .partition(|suite| matches!(suite.environment, Environment::Rcc(_)));
    surviving_suites.append(&mut rcc_setup(global_config, rcc_suites)?);
    sort_suites_by_id(&mut surviving_suites);
    Ok(surviving_suites)
}

fn adjust_rcc_binary_permissions(executable_path: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` read and execute access to {executable_path}");
    run_icacls_command(vec![executable_path.as_str(), "/grant", "Users:(RX)"]).context(format!(
        "Adjusting permissions of {executable_path} for group `Users` failed",
    ))
}

fn clear_rcc_setup_working_directory(working_directory: &Utf8Path) -> Result<()> {
    if working_directory.exists() {
        remove_dir_all(working_directory).context(format!(
            "Failed to remove working directory for RCC setup: {working_directory}"
        ))?;
    }
    create_dir_all(working_directory).context(format!(
        "Failed to create working directory for RCC setup: {working_directory}"
    ))
}

fn rcc_setup_working_directory(working_directory: &Utf8Path) -> Utf8PathBuf {
    working_directory.join("rcc_setup")
}

fn adjust_rcc_profile_permissions(profile_path: &Utf8Path) -> Result<()> {
    debug!("Granting group `Users` read access to {profile_path}");
    run_icacls_command(vec![profile_path.as_str(), "/grant", "Users:(R)"]).context(format!(
        "Adjusting permissions of {profile_path} for group `Users` failed",
    ))
}

fn rcc_setup(global_config: &GlobalConfig, rcc_suites: Vec<Suite>) -> Result<Vec<Suite>> {
    let mut rcc_setup_failures = RCCSetupFailures {
        telemetry_disabling: vec![],
        profile_configuring: vec![],
        long_path_support: vec![],
        shared_holotree: vec![],
        holotree_init: vec![],
    };

    debug!("Disabling RCC telemetry");
    let (mut sucessful_suites, mut failed_suites) =
        disable_rcc_telemetry(global_config, rcc_suites)
            .context("Disabling RCC telemetry failed")?;
    rcc_setup_failures.telemetry_disabling =
        failed_suites.into_iter().map(|suite| suite.id).collect();
    if !rcc_setup_failures.telemetry_disabling.is_empty() {
        error!(
            "Dropping the following suites due to RCC telemetry disabling failure: {}",
            rcc_setup_failures.telemetry_disabling.join(", ")
        );
    }

    debug!("Configuring RCC profile");
    (sucessful_suites, failed_suites) = configure_rcc_profile(global_config, sucessful_suites)
        .context("Configuring RCC profile failed")?;
    rcc_setup_failures.profile_configuring =
        failed_suites.into_iter().map(|suite| suite.id).collect();
    if !rcc_setup_failures.profile_configuring.is_empty() {
        error!(
            "Dropping the following suites due to profile configuring failure: {}",
            rcc_setup_failures.profile_configuring.join(", ")
        );
    }

    debug!("Enabling support for long paths");
    (sucessful_suites, failed_suites) = enable_long_path_support(global_config, sucessful_suites)
        .context("Enabling support for long paths failed")?;
    rcc_setup_failures.long_path_support =
        failed_suites.into_iter().map(|suite| suite.id).collect();
    if !rcc_setup_failures.long_path_support.is_empty() {
        error!(
            "Dropping the following suites due to long path support enabling failure: {}",
            rcc_setup_failures.long_path_support.join(", ")
        );
    }

    debug!("Initializing shared holotree");
    (sucessful_suites, failed_suites) = shared_holotree_init(global_config, sucessful_suites)
        .context("Shared holotree initialization failed")?;
    rcc_setup_failures.shared_holotree = failed_suites.into_iter().map(|suite| suite.id).collect();
    if !rcc_setup_failures.shared_holotree.is_empty() {
        error!(
            "Dropping the following suites due to shared holotree initialization failure: {}",
            rcc_setup_failures.shared_holotree.join(", ")
        );
    }

    debug!("Initializing holotree");
    (sucessful_suites, failed_suites) =
        holotree_init(global_config, sucessful_suites).context("Holotree initialization failed")?;
    rcc_setup_failures.holotree_init = failed_suites.into_iter().map(|suite| suite.id).collect();
    if !rcc_setup_failures.holotree_init.is_empty() {
        error!(
            "Dropping the following suites due to holotree initialization failure: {}",
            rcc_setup_failures.holotree_init.join(", ")
        );
    }

    let path = global_config
        .results_directory
        .join("rcc_setup_failures.json");
    rcc_setup_failures.write(path, &global_config.results_directory_locker)?;

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
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec![
                "configure".into(),
                "identity".into(),
                "--do-not-track".into(),
            ],
        },
        "telemetry_disabling",
    )
}

fn configure_rcc_profile(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    match &global_config.rcc_config.profile_config {
        RCCProfileConfig::Default => configure_default_rcc_profile(global_config, suites),
        RCCProfileConfig::Custom(custom_rcc_profile_config) => {
            configure_custom_rcc_profile(custom_rcc_profile_config, global_config, suites)
        }
    }
}

fn configure_default_rcc_profile(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec![
                "configuration".into(),
                "switch".into(),
                "--noprofile".into(),
            ],
        },
        "default_profile_switch",
    )
}

fn configure_custom_rcc_profile(
    custom_rcc_profile_config: &CustomRCCProfileConfig,
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    let (sucessful_suites_import, failed_suites_import) = run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec![
                "configuration".into(),
                "import".into(),
                "--filename".into(),
                custom_rcc_profile_config.path.to_string(),
            ],
        },
        "custom_profile_import",
    )?;
    let (sucessful_suites_switch, failed_suites_switch) = run_command_spec_per_session(
        global_config,
        sucessful_suites_import,
        &CommandSpec {
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec![
                "configuration".into(),
                "switch".into(),
                "--profile".into(),
                custom_rcc_profile_config.name.to_string(),
            ],
        },
        "custom_profile_switch",
    )?;
    let mut failed_suites = vec![];
    failed_suites.extend(failed_suites_import);
    failed_suites.extend(failed_suites_switch);
    Ok((sucessful_suites_switch, failed_suites))
}

fn enable_long_path_support(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    run_command_spec_once_in_current_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec!["configure".into(), "longpaths".into(), "--enable".into()],
        },
        "long_path_support_enabling",
    )
}

fn shared_holotree_init(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    run_command_spec_once_in_current_session(
        global_config,
        suites,
        &CommandSpec {
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec![
                "holotree".into(),
                "shared".into(),
                "--enable".into(),
                "--once".into(),
            ],
        },
        "shared_holotree_init",
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
            executable: global_config.rcc_config.binary_path.to_string(),
            arguments: vec!["holotree".into(), "init".into()],
        },
        "holotree_initialization",
    )
}

fn run_command_spec_once_in_current_session(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
    command_spec: &CommandSpec,
    id: &str,
) -> Result<(Vec<Suite>, Vec<Suite>)> {
    Ok(
        if run_command_spec_in_session(
            &Session::Current(CurrentSession {}),
            &RunSpec {
                id: &format!("robotmk_{id}"),
                command_spec,
                base_path: &rcc_setup_working_directory(&global_config.working_directory).join(id),
                timeout: 120,
                cancellation_token: &global_config.cancellation_token,
            },
        )? {
            (suites, vec![])
        } else {
            (vec![], suites)
        },
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
                cancellation_token: &global_config.cancellation_token,
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
    let run_outcome = match session.run(run_spec).context(format!(
        "Failed to run {} for `{session}`",
        run_spec.command_spec
    )) {
        Ok(run_outcome) => run_outcome,
        Err(error) => {
            log_and_return_error(error);
            return Ok(false);
        }
    };
    match run_outcome {
        RunOutcome::Exited(exit_code) => match exit_code {
            Some(0) => {
                debug!("{} for `{session}` successful", run_spec.command_spec);
                Ok(true)
            }
            Some(_) => {
                error!(
                    "{} for `{session}` exited non-successfully",
                    run_spec.command_spec
                );
                Ok(false)
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
