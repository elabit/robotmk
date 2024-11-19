use super::api::{run_steps, SetupStep};
use super::{directories, rcc};
use crate::internal_config::{sort_plans_by_grouping, GlobalConfig, Plan};
use robotmk::results::SetupFailure;
use robotmk::termination::Cancelled;

pub fn run(
    config: &GlobalConfig,
    mut plans: Vec<Plan>,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut failures = Vec::new();
    for gatherer in STEPS {
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

type Gatherer = fn(&GlobalConfig, Vec<Plan>) -> Vec<(Box<dyn SetupStep>, Vec<Plan>)>;
#[cfg(unix)]
type Steps = [Gatherer; 10];
#[cfg(windows)]
type Steps = [Gatherer; 16];

const STEPS: Steps = [
    directories::gather_managed_directories,
    #[cfg(windows)]
    directories::gather_robocorp_home_base,
    #[cfg(windows)]
    directories::gather_robocorp_home_per_user,
    directories::gather_plan_working_directories,
    directories::gather_environment_building_directories,
    directories::gather_rcc_working_base,
    #[cfg(windows)]
    directories::gather_rcc_longpath_directory,
    directories::gather_rcc_working_per_user,
    #[cfg(windows)]
    rcc::gather_rcc_binary_permissions,
    #[cfg(windows)]
    rcc::gather_rcc_profile_permissions,
    rcc::gather_disable_rcc_telemetry,
    rcc::gather_configure_default_rcc_profile,
    rcc::gather_import_custom_rcc_profile,
    rcc::gather_switch_to_custom_rcc_profile,
    #[cfg(windows)]
    rcc::gather_enable_rcc_long_path_support,
    rcc::gather_disable_rcc_shared_holotree,
];
