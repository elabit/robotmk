pub mod general;
pub mod rcc;
pub mod unpack_managed;

use crate::internal_config::Plan;
use robotmk::session::Session;
use std::collections::HashMap;

#[cfg(windows)]
mod windows {
    use camino::Utf8Path;
    use core::ffi::c_void;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows::core::{HSTRING, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{LocalFree, GENERIC_ALL, HLOCAL};
    use windows::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
        GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, TRUSTEE_IS_NAME, TRUSTEE_IS_USER,
        TRUSTEE_W,
    };
    use windows::Win32::Security::{
        ACL, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, OBJECT_INHERIT_ACE,
        PSECURITY_DESCRIPTOR,
    };

    pub struct OwnedLocalAlloc<T>(pub *mut T);

    impl<T> Default for OwnedLocalAlloc<T> {
        fn default() -> Self {
            unsafe { std::mem::zeroed() }
        }
    }

    impl<T> Drop for OwnedLocalAlloc<T> {
        fn drop(&mut self) {
            if !self.0.is_null() {
                let _ = unsafe { LocalFree(HLOCAL(self.0 as *mut c_void)) };
                self.0 = null_mut();
            }
        }
    }

    pub fn grant_full_access(user: &str, target_path: &Utf8Path) -> anyhow::Result<()> {
        grant_access(user, target_path, GENERIC_ALL.0).map_err(|e| {
            let message =
                format!("Adjusting permissions of {target_path} for user `{user}` failed");
            e.context(message)
        })
    }

    pub fn grant_access(
        user: &str,
        path: &Utf8Path,
        access_permissions: u32,
    ) -> anyhow::Result<()> {
        let psecurity = OwnedLocalAlloc::<PSECURITY_DESCRIPTOR>(null_mut());
        let mut oldacl = null_mut();
        unsafe {
            GetNamedSecurityInfoW(
                PCWSTR(HSTRING::from(path.as_std_path()).as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(&mut oldacl),
                None,
                psecurity.0,
            )
        }?;
        let mut newdacl = OwnedLocalAlloc(null_mut());
        let mut os_user: Vec<u16> = OsStr::new(user).encode_wide().chain([0]).collect();
        let ea = &[EXPLICIT_ACCESS_W {
            grfAccessPermissions: access_permissions,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_NAME,
                TrusteeType: TRUSTEE_IS_USER,
                ptstrName: PWSTR(os_user.as_mut_ptr()),
            },
        }];
        unsafe { SetEntriesInAclW(Some(ea), Some(oldacl), &mut newdacl.0) }?;
        Ok(unsafe {
            SetNamedSecurityInfoW(
                PCWSTR(HSTRING::from(path.as_std_path()).as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(newdacl.0 as *const ACL),
                None,
            )
        }?)
    }
}

fn plans_by_sessions(plans: Vec<Plan>) -> HashMap<Session, Vec<Plan>> {
    let mut plans_by_session = HashMap::new();
    for plan in plans {
        plans_by_session
            .entry(plan.session.clone())
            .or_insert(vec![])
            .push(plan);
    }
    plans_by_session
}
