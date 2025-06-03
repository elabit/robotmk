use super::api::{StepWithPlans, run_steps};
use super::{directories, rcc, unpack_managed};
use crate::internal_config::{GlobalConfig, Plan, sort_plans_by_grouping};
use log::info;
use robotmk::results::SetupFailure;
use robotmk::termination::Cancelled;

pub fn run(
    config: &GlobalConfig,
    mut plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut failures = Vec::new();
    for (gatherer, stage) in STEPS {
        info!("Running setup stage: {stage}");
        plans = {
            let plan_count = plans.len();
            let setup_steps = gatherer(config, plans);
            assert_eq!(
                plan_count,
                setup_steps.iter().map(|s| s.1.len()).sum::<usize>()
            );
            let (surviving_plans, current_errors) =
                run_steps(setup_steps, &config.cancellation_token)?;
            failures.extend(current_errors);
            surviving_plans
        };
    }
    sort_plans_by_grouping(&mut plans);
    Ok((plans, failures))
}

type Gatherer = fn(&GlobalConfig, Vec<Plan>) -> Vec<StepWithPlans>;
#[cfg(unix)]
type Steps = [(Gatherer, &'static str); 14];
#[cfg(windows)]
type Steps = [(Gatherer, &'static str); 21];

const STEPS: Steps = [
    #[cfg(windows)]
    (
        super::long_path_support::gather_long_path_support,
        "Long path support",
    ),
    (
        directories::gather_managed_directories,
        "Managed directories",
    ),
    #[cfg(windows)]
    (
        directories::gather_robocorp_home_base,
        "ROBOCORP_HOME base directory",
    ),
    // It is unclear why this is needed. Without it, non-admin users cannot build RCC environments
    // (with ROBOCORP_HOME set). Micromamba crashes with the following error:
    // info     libmamba ****************** Backtrace Start ******************
    // debug    libmamba Loading configuration
    // trace    libmamba Compute configurable 'create_base'
    // trace    libmamba Compute configurable 'no_env'
    // trace    libmamba Compute configurable 'no_rc'
    // trace    libmamba Compute configurable 'rc_files'
    // trace    libmamba Compute configurable 'root_prefix'
    // trace    libmamba Compute configurable 'envs_dirs'
    // critical libmamba weakly_canonical: Access is denied.: "C:\rmk\rcc_home\vagrant2\envs"
    // info     libmamba ****************** Backtrace End ********************
    #[cfg(windows)]
    (
        directories::gather_robocorp_base_read_access,
        "Read access to ROBOCORP_HOME base directory",
    ),
    #[cfg(windows)]
    (
        directories::gather_robocorp_home_per_user,
        "User-specific ROBOCORP_HOME directories",
    ),
    (directories::gather_conda_base, "Conda base directory"),
    #[cfg(windows)]
    (
        directories::gather_conda_base_read_and_execute_access,
        "Read and execute access to conda base directory",
    ),
    (
        directories::gather_plan_working_directories,
        "Plan working directories",
    ),
    (
        directories::gather_rcc_environment_building_directories,
        "RCC environment building directories",
    ),
    (
        directories::gather_conda_environment_building_directories,
        "Conda environment building directories",
    ),
    (
        directories::gather_rcc_working_base,
        "Base working directory for RCC setup steps",
    ),
    (
        directories::gather_rcc_working_per_user,
        "User-specififc working directories for RCC setup steps",
    ),
    #[cfg(windows)]
    (rcc::gather_rcc_binary_permissions, "RCC binary permissions"),
    #[cfg(windows)]
    (
        rcc::gather_rcc_profile_permissions,
        "RCC profile permissions",
    ),
    (rcc::gather_disable_rcc_telemetry, "Disable RCC telemetry"),
    (
        rcc::gather_configure_default_rcc_profile,
        "Configure default RCC profile",
    ),
    (
        rcc::gather_import_custom_rcc_profile,
        "Import custom RCC profile",
    ),
    (
        rcc::gather_switch_to_custom_rcc_profile,
        "Switch to custom RCC profile",
    ),
    (
        rcc::gather_disable_rcc_shared_holotree,
        "Disable RCC shared holotrees",
    ),
    (
        super::conda::gather_copy_micromamba_binary,
        "Copy micromamba binary",
    ),
    (unpack_managed::gather, "Unpack managed robots"),
];
