use crate::internal_config::Plan;
use log::{debug, error};
use robotmk::results::SetupFailure;
use robotmk::termination::Cancelled;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Error {
    summary: String,
    cause: anyhow::Error,
}

impl Error {
    pub fn new(summary: String, cause: anyhow::Error) -> Error {
        Error { summary, cause }
    }
}

pub trait SetupStep {
    fn label(&self) -> String;
    fn setup(&self) -> Result<(), Error>;
}

pub type StepWithPlans = (Box<dyn SetupStep>, Vec<Plan>);

struct SetupStepNoOp {}

impl SetupStep for SetupStepNoOp {
    fn label(&self) -> String {
        "No-op".into()
    }

    fn setup(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn skip(plans: Vec<Plan>) -> StepWithPlans {
    (Box::new(SetupStepNoOp {}), plans)
}

pub fn run_steps(
    steps: Vec<StepWithPlans>,
    cancellation_token: &CancellationToken,
) -> Result<(Vec<Plan>, Vec<SetupFailure>), Cancelled> {
    let mut plans = Vec::new();
    let mut errors = Vec::new();
    for (step, affected_plans) in steps.into_iter() {
        if cancellation_token.is_cancelled() {
            return Err(Cancelled);
        }
        if affected_plans.is_empty() {
            debug!("Setup step `{}` affects no plans, skipping", step.label());
            continue;
        }
        debug!(
            "Plan(s) {plan_ids}: {label}",
            label = step.label(),
            plan_ids = affected_plans
                .iter()
                .map(|plan| plan.id.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );
        match step.setup() {
            Ok(()) => {
                plans.extend(affected_plans);
            }
            Err(err) => {
                for plan in affected_plans {
                    error!(
                        "Plan {}: {}. Plan won't be scheduled.\nCaused by: {:?}",
                        plan.id, err.summary, err.cause,
                    );
                    errors.push(SetupFailure {
                        plan_id: plan.id.clone(),
                        summary: err.summary.clone(),
                        details: format!("{:?}", err.cause),
                    });
                }
            }
        }
    }
    Ok((plans, errors))
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;

    use super::*;
    use crate::internal_config::{GroupAffiliation, Source};
    use robotmk::config::{PlanMetadata, RetryStrategy, WorkingDirectoryCleanupConfig};
    use robotmk::environment::{Environment, SystemEnvironment};
    use robotmk::lock::Locker;
    use robotmk::rf::robot::Robot;
    use robotmk::section::Host;
    use robotmk::session::{CurrentSession, Session};
    use tokio_util::sync::CancellationToken;

    struct SetupStepOk {}

    impl SetupStep for SetupStepOk {
        fn label(&self) -> String {
            "Ok".into()
        }

        fn setup(&self) -> Result<(), Error> {
            Ok(())
        }
    }

    struct SetupStepError {}

    impl SetupStep for SetupStepError {
        fn label(&self) -> String {
            "Error".into()
        }

        fn setup(&self) -> Result<(), Error> {
            Err(Error::new("Error".into(), anyhow::anyhow!("Error")))
        }
    }

    #[test]
    fn test_run_steps() {
        let plan_bluerpint = Plan {
            id: String::default(),
            source: Source::Manual,
            working_directory: Utf8PathBuf::default(),
            results_file: Utf8PathBuf::default(),
            timeout: u64::default(),
            robot: Robot {
                robot_target: Utf8PathBuf::default(),
                command_line_args: Vec::default(),
                envs_rendered_obfuscated: Vec::default(),
                n_attempts_max: usize::default(),
                retry_strategy: RetryStrategy::Incremental,
            },
            environment: Environment::System(SystemEnvironment {}),
            session: Session::Current(CurrentSession {}),
            working_directory_cleanup_config: WorkingDirectoryCleanupConfig::MaxAgeSecs(
                u64::default(),
            ),
            cancellation_token: CancellationToken::new(),
            host: Host::Source,
            results_directory_locker: Locker::new(Utf8PathBuf::default(), None),
            metadata: PlanMetadata {
                application: String::default(),
                suite_name: String::default(),
                variant: String::default(),
            },
            group_affiliation: GroupAffiliation {
                group_index: usize::default(),
                position_in_group: usize::default(),
                execution_interval: u64::default(),
            },
        };
        let mut plan_ok = plan_bluerpint.clone();
        plan_ok.id = "ok".into();
        let mut plan_error = plan_bluerpint.clone();
        plan_error.id = "error".into();

        let (passed_plans, errors) = run_steps(
            vec![
                (Box::new(SetupStepOk {}), vec![plan_ok]),
                (Box::new(SetupStepError {}), vec![plan_error]),
            ],
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(passed_plans.len(), 1);
        assert_eq!(passed_plans[0].id, "ok");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].plan_id, "error");
    }
}
