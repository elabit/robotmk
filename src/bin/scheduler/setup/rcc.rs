use super::all_configured_users;
use super::icacls::run_icacls_command;
use crate::internal_config::{sort_suites_by_grouping, GlobalConfig, Suite};
use crate::logging::log_and_return_error;
use robotmk::command_spec::CommandSpec;
use robotmk::config::{CustomRCCProfileConfig, RCCConfig, RCCProfileConfig};
use robotmk::environment::Environment;
use robotmk::results::RCCSetupFailures;
use robotmk::section::WriteSection;
use robotmk::session::{CurrentSession, RunSpec, Session};
use robotmk::termination::{Cancelled, Outcome};

use anyhow::{Context, Result as AnyhowResult};
use camino::{Utf8Path, Utf8PathBuf};
use log::{debug, error};
use std::collections::HashMap;
use std::fs::{create_dir_all, remove_dir_all};
use std::vec;

pub fn setup(
    global_config: &GlobalConfig,
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> AnyhowResult<Vec<Suite>> {
    let (rcc_suites, mut surviving_suites): (Vec<Suite>, Vec<Suite>) = suites
        .into_iter()
        .partition(|suite| matches!(suite.environment, Environment::Rcc(_)));
    let all_configured_users_rcc = all_configured_users(rcc_suites.iter());

    for user_name in &all_configured_users_rcc {
        adjust_rcc_binary_permissions(&rcc_config.binary_path, user_name)
            .context("Failed to adjust permissions of RCC binary")?;
    }
    clear_rcc_setup_working_directory(&rcc_setup_working_directory(
        &global_config.working_directory,
    ))?;
    if let RCCProfileConfig::Custom(custom_rcc_profile_config) = &rcc_config.profile_config {
        for user_name in &all_configured_users_rcc {
            adjust_rcc_profile_permissions(&custom_rcc_profile_config.path, user_name)
                .context("Failed to adjust permissions of RCC profile")?;
        }
    }

    surviving_suites.append(&mut rcc_setup(global_config, rcc_config, rcc_suites)?);
    sort_suites_by_grouping(&mut surviving_suites);
    Ok(surviving_suites)
}

fn adjust_rcc_binary_permissions(executable_path: &Utf8Path, user_name: &str) -> AnyhowResult<()> {
    debug!("Granting user `{user_name}` read and execute access to {executable_path}");
    run_icacls_command(vec![
        executable_path.as_str(),
        "/grant",
        &format!("{user_name}:(RX)"),
    ])
    .context(format!(
        "Adjusting permissions of {executable_path} for user `{user_name}` failed",
    ))
}

fn clear_rcc_setup_working_directory(working_directory: &Utf8Path) -> AnyhowResult<()> {
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

fn adjust_rcc_profile_permissions(profile_path: &Utf8Path, user_name: &str) -> AnyhowResult<()> {
    debug!("Granting user `{user_name}` read access to {profile_path}");
    run_icacls_command(vec![
        profile_path.as_str(),
        "/grant",
        &format!("{user_name}:(R)"),
    ])
    .context(format!(
        "Adjusting permissions of {profile_path} for user `{user_name}` failed",
    ))
}

fn rcc_setup(
    global_config: &GlobalConfig,
    rcc_config: &RCCConfig,
    rcc_suites: Vec<Suite>,
) -> AnyhowResult<Vec<Suite>> {
    let mut sucessful_suites: Vec<Suite>;
    let mut rcc_setup_failures = RCCSetupFailures {
        telemetry_disabling: HashMap::new(),
        profile_configuring: HashMap::new(),
        long_path_support: HashMap::new(),
        shared_holotree: HashMap::new(),
        holotree_init: HashMap::new(),
    };

    debug!("Disabling RCC telemetry");
    (sucessful_suites, rcc_setup_failures.telemetry_disabling) =
        disable_rcc_telemetry(global_config, rcc_config, rcc_suites)
            .context("Received termination signal while disabling RCC telemetry")?;
    if !rcc_setup_failures.telemetry_disabling.is_empty() {
        error!(
            "Dropping the following suites due to RCC telemetry disabling failure: {}",
            rcc_setup_failures_human_readable(rcc_setup_failures.telemetry_disabling.keys())
        );
    }

    debug!("Configuring RCC profile");
    (sucessful_suites, rcc_setup_failures.profile_configuring) =
        configure_rcc_profile(global_config, rcc_config, sucessful_suites)
            .context("Received termination signal while configuring RCC profile")?;
    if !rcc_setup_failures.profile_configuring.is_empty() {
        error!(
            "Dropping the following suites due to profile configuring failure: {}",
            rcc_setup_failures_human_readable(rcc_setup_failures.profile_configuring.keys())
        );
    }

    debug!("Enabling support for long paths");
    (sucessful_suites, rcc_setup_failures.long_path_support) =
        enable_long_path_support(global_config, rcc_config, sucessful_suites)
            .context("Received termination signal while enabling support for long paths")?;
    if !rcc_setup_failures.long_path_support.is_empty() {
        error!(
            "Dropping the following suites due to long path support enabling failure: {}",
            rcc_setup_failures_human_readable(rcc_setup_failures.long_path_support.keys())
        );
    }

    debug!("Initializing shared holotree");
    (sucessful_suites, rcc_setup_failures.shared_holotree) =
        shared_holotree_init(global_config, rcc_config, sucessful_suites)
            .context("Received termination signal while initializing shared holotree")?;
    if !rcc_setup_failures.shared_holotree.is_empty() {
        error!(
            "Dropping the following suites due to shared holotree initialization failure: {}",
            rcc_setup_failures_human_readable(rcc_setup_failures.shared_holotree.keys())
        );
    }

    debug!("Initializing holotree");
    (sucessful_suites, rcc_setup_failures.holotree_init) =
        holotree_init(global_config, rcc_config, sucessful_suites)
            .context("Received termination signal while initializing holotree")?;
    if !rcc_setup_failures.holotree_init.is_empty() {
        error!(
            "Dropping the following suites due to holotree initialization failure: {}",
            rcc_setup_failures_human_readable(rcc_setup_failures.holotree_init.keys())
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
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_config.binary_path.to_string(),
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
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    match &rcc_config.profile_config {
        RCCProfileConfig::Default => {
            configure_default_rcc_profile(global_config, &rcc_config.binary_path, suites)
        }
        RCCProfileConfig::Custom(custom_rcc_profile_config) => configure_custom_rcc_profile(
            global_config,
            &rcc_config.binary_path,
            custom_rcc_profile_config,
            suites,
        ),
    }
}

fn configure_default_rcc_profile(
    global_config: &GlobalConfig,
    rcc_binary_path: &Utf8Path,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_binary_path.to_string(),
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
    global_config: &GlobalConfig,
    rcc_binary_path: &Utf8Path,
    custom_rcc_profile_config: &CustomRCCProfileConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    let (sucessful_suites_import, failed_suites_import) = run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_binary_path.to_string(),
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
            executable: rcc_binary_path.to_string(),
            arguments: vec![
                "configuration".into(),
                "switch".into(),
                "--profile".into(),
                custom_rcc_profile_config.name.to_string(),
            ],
        },
        "custom_profile_switch",
    )?;
    let mut failed_suites = HashMap::new();
    failed_suites.extend(failed_suites_import);
    failed_suites.extend(failed_suites_switch);
    Ok((sucessful_suites_switch, failed_suites))
}

fn enable_long_path_support(
    global_config: &GlobalConfig,
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    run_command_spec_once_in_current_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_config.binary_path.to_string(),
            arguments: vec!["configure".into(), "longpaths".into(), "--enable".into()],
        },
        "long_path_support_enabling",
    )
}

fn shared_holotree_init(
    global_config: &GlobalConfig,
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    run_command_spec_once_in_current_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_config.binary_path.to_string(),
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
    rcc_config: &RCCConfig,
    suites: Vec<Suite>,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    run_command_spec_per_session(
        global_config,
        suites,
        &CommandSpec {
            executable: rcc_config.binary_path.to_string(),
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
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    Ok(
        match run_command_spec_in_session(
            &Session::Current(CurrentSession {}),
            &RunSpec {
                id: &format!("robotmk_{id}"),
                command_spec,
                base_path: &rcc_setup_working_directory(&global_config.working_directory).join(id),
                timeout: 120,
                cancellation_token: &global_config.cancellation_token,
            },
        )? {
            None => (suites, HashMap::new()),
            Some(error_msg) => (
                vec![],
                HashMap::from_iter(
                    suites
                        .into_iter()
                        .map(|suite| (suite.id, error_msg.clone())),
                ),
            ),
        },
    )
}

fn run_command_spec_per_session(
    global_config: &GlobalConfig,
    suites: Vec<Suite>,
    command_spec: &CommandSpec,
    id: &str,
) -> Result<(Vec<Suite>, HashMap<String, String>), Cancelled> {
    let mut suites_by_session = HashMap::new();
    for suite in suites {
        suites_by_session
            .entry(suite.session.clone())
            .or_insert(vec![])
            .push(suite);
    }
    let mut succesful_suites = vec![];
    let mut failed_suites = HashMap::new();

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
        match run_command_spec_in_session(
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
            Some(error_msg) => {
                for suite in suites {
                    failed_suites.insert(suite.id, error_msg.clone());
                }
            }
            None => succesful_suites.extend(suites),
        }
    }

    Ok((succesful_suites, failed_suites))
}

fn run_command_spec_in_session(
    session: &Session,
    run_spec: &RunSpec,
) -> Result<Option<String>, Cancelled> {
    let run_outcome = match session.run(run_spec).context(format!(
        "Failed to run {} for `{session}`",
        run_spec.command_spec
    )) {
        Ok(run_outcome) => run_outcome,
        Err(error) => {
            let error = log_and_return_error(error);
            return Ok(Some(format!("{error:?}")));
        }
    };
    let exit_code = match run_outcome {
        Outcome::Completed(exit_code) => exit_code,
        Outcome::Timeout => {
            error!("{} for `{session}` timed out", run_spec.command_spec);
            return Ok(Some("Timeout".into()));
        }
        Outcome::Cancel => {
            error!("{} for `{session}` cancelled", run_spec.command_spec);
            return Err(Cancelled {});
        }
    };
    if exit_code == 0 {
        debug!("{} for `{session}` successful", run_spec.command_spec);
        Ok(None)
    } else {
        error!(
            "{} for `{session}` exited non-successfully",
            run_spec.command_spec
        );
        Ok(Some(
            "Non-zero exit code, see stdio logs for details".into(),
        ))
    }
}

fn rcc_setup_failures_human_readable<'a>(failures: impl Iterator<Item = &'a String>) -> String {
    failures
        .map(|suite_id| suite_id.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}
