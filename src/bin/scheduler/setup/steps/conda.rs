#![cfg(windows)]

use super::api::{self, SetupStep, StepWithPlans, skip};
use super::partition_into_conda_and_other_plans;

use crate::internal_config::{GlobalConfig, Plan};
use crate::setup::windows_permissions::run_icacls_command;

use robotmk::session::Session;

use camino::Utf8PathBuf;

struct StepFilePermissions {
    target: Utf8PathBuf,
    sid: String,
    icacls_permissions: String,
}

impl SetupStep for StepFilePermissions {
    fn label(&self) -> String {
        format!(
            "Grant SID {sid} permissions `{permissions}` for {target}",
            sid = self.sid,
            permissions = &self.icacls_permissions,
            target = &self.target,
        )
    }

    fn setup(&self) -> Result<(), api::Error> {
        run_icacls_command([
            self.target.as_str(),
            "/grant",
            &format!("{}:{}", &self.sid, self.icacls_permissions),
        ])
        .map_err(|err| {
            api::Error::new(
                format!(
                    "Adjusting permissions of {} for SID `{}` failed",
                    self.target, &self.sid
                ),
                err,
            )
        })
    }
}

pub fn gather_micromamba_binary_permissions(
    config: &GlobalConfig,
    plans: Vec<Plan>,
) -> Vec<StepWithPlans> {
    let (conda_plans, other_plans): (Vec<Plan>, Vec<Plan>) =
        partition_into_conda_and_other_plans(plans);
    let (conda_plans_in_current_session, conda_plans_in_other_session): (Vec<Plan>, Vec<Plan>) =
        conda_plans
            .into_iter()
            .partition(|plan| matches!(plan.session, Session::Current(_)));
    vec![
        skip(other_plans),
        skip(conda_plans_in_current_session),
        (
            Box::new(StepFilePermissions {
                target: config.conda_config.micromamba_binary_path.clone(),
                sid: "*S-1-5-32-545".into(), // Users (https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/understand-security-identifiers)
                icacls_permissions: "(RX)".into(),
            }),
            conda_plans_in_other_session,
        ),
    ]
}
