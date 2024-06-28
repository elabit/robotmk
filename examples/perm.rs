#![cfg(windows)]
use camino::Utf8Path;
use std::fs::{create_dir, remove_dir_all};
use std::process::Command;

use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use std::os::windows::ffi::OsStrExt;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{LocalFree, GENERIC_ALL, HLOCAL};
use windows::Win32::Security::Authorization::{
    SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE, SET_ACCESS,
    SE_FILE_OBJECT, TRUSTEE_IS_NAME, TRUSTEE_IS_USER, TRUSTEE_W,
};
use windows::Win32::Security::{ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE};

#[repr(transparent)]
pub struct OwnedLocalAlloc<T>(pub T);

impl<T> Default for OwnedLocalAlloc<T> {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl<T> Drop for OwnedLocalAlloc<T> {
    fn drop(&mut self) {
        unsafe {
            let ptr: HLOCAL = std::mem::transmute_copy(self);
            if !ptr.0.is_null() {
                let _ = LocalFree(ptr);
            }
        }
    }
}

impl<T> Deref for OwnedLocalAlloc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for OwnedLocalAlloc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn print_permissions(path: &Utf8Path) {
    let mut icacls_command = Command::new("icacls.exe");
    icacls_command.arg(path);
    let output = String::from_utf8(icacls_command.output().unwrap().stdout).unwrap();
    println!("\n{output}");
}

fn main() {
    // Input
    let user = "vagrant";
    let path = Utf8Path::new("C:\\Users\\vagrant\\Desktop\\t");

    // Reset
    if path.exists() {
        remove_dir_all(path).unwrap();
    }
    create_dir(path).unwrap();

    print_permissions(path);

    let mut newdacl = OwnedLocalAlloc::<*mut ACL>::default();
    let mut os_user: Vec<u16> = OsStr::new(user).encode_wide().chain([0]).collect();
    let ea = [EXPLICIT_ACCESS_W {
        grfAccessPermissions: GENERIC_ALL.0,
        grfAccessMode: SET_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_NAME,
            TrusteeType: TRUSTEE_IS_USER,
            ptstrName: PWSTR(os_user.as_mut_ptr()),
        },
    }];

    unsafe { SetEntriesInAclW(Some(&ea), None, &mut *newdacl) }.unwrap();
    println!("{:?}\n", unsafe { **newdacl });
    unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(HSTRING::from(path.as_std_path()).as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(*newdacl as *const _),
            None,
        )
    }
    .unwrap();

    print_permissions(path);
}
