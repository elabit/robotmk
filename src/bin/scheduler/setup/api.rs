use crate::internal_config::Plan;
use robotmk::results::SetupFailure;

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
    fn setup(&self) -> Result<(), Error>;
}

pub type StepWithPlans = (Box<dyn SetupStep>, Vec<Plan>);

struct SetupStepSuccess {}

impl SetupStep for SetupStepSuccess {
    fn setup(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn skip(plans: Vec<Plan>) -> (Box<dyn SetupStep>, Vec<Plan>) {
    (Box::new(SetupStepSuccess {}), plans)
}

pub fn run_steps(steps: Vec<StepWithPlans>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut plans = Vec::new();
    let mut errors = Vec::new();
    for (step, affected_plans) in steps.into_iter() {
        match step.setup() {
            Ok(()) => {
                plans.extend(affected_plans);
            }
            Err(err) => {
                for plan in &plans {
                    log::error!(
                        "Plan {}: {}. Plan won't be scheduled.\nCaused by: {:?}",
                        plan.id,
                        err.summary,
                        err.cause,
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
    (plans, errors)
}
