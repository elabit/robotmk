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
                for plan in &plans {
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
