#![cfg(windows)]
use anyhow::{Context, bail};
use camino::{Utf8Path, Utf8PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use windows::{
    Win32::Security::{
        AllocateAndInitializeSid, FreeSid, OWNER_SECURITY_INFORMATION, SID_IDENTIFIER_AUTHORITY,
    },
    Win32::Security::{
        Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW},
        PSID,
    },
    core::PCWSTR,
};

const SECURITY_NT_AUTHORITY: SID_IDENTIFIER_AUTHORITY = SID_IDENTIFIER_AUTHORITY {
    Value: [0, 0, 0, 0, 0, 5],
};
const SECURITY_BUILTIN_DOMAIN_RID: u32 = 32;
const DOMAIN_ALIAS_RID_ADMINS: u32 = 544;

pub fn run_icacls_command<'a>(
    target_path: &Utf8Path,
    further_arguments: impl IntoIterator<Item = &'a str>,
) -> anyhow::Result<()> {
    let mut icacls_args = vec![make_long_path(target_path)];
    icacls_args.extend(further_arguments.into_iter().map(|s| s.to_string()));
    run_command("icacls.exe", icacls_args)
}

pub fn grant_full_access(sid: &str, target_path: &Utf8Path) -> anyhow::Result<()> {
    run_icacls_command(target_path, ["/grant", &format!("{sid}:(OI)(CI)F"), "/T"]).map_err(|e| {
        let message = format!("Adjusting permissions of {target_path} for SID `{sid}` failed");
        e.context(message)
    })
}

pub fn reset_access(target_path: &Utf8Path) -> anyhow::Result<()> {
    run_icacls_command(target_path, ["/reset", "/T"]).map_err(|e| {
        let message = format!("Resetting permissions of {target_path} failed");
        e.context(message)
    })
}

pub fn transfer_directory_ownership_to_admin_group_recursive(
    target_path: &Utf8Path,
) -> anyhow::Result<()> {
    let admin_sid = build_admin_sid().map_err(|e| {
        e.context(format!(
            "Building administrator SID for transferring ownership of {target_path} failed"
        ))
    })?;
    for entry in walkdir::WalkDir::new(target_path) {
        set_owner(
            &Utf8PathBuf::try_from(
                entry
                    .context(format!(
                        "Traversing directory {target_path} for transferring ownership failed",
                        target_path = target_path,
                    ))?
                    .path()
                    .to_path_buf(),
            )
            .context(format!(
                "Converting path to Utf8PathBuf failed for transferring ownership of {target_path}",
                target_path = target_path,
            ))?,
            admin_sid,
        )?;
    }
    unsafe {
        FreeSid(admin_sid);
    }
    Ok(())
}

fn make_long_path(path: &Utf8Path) -> String {
    format!("\\\\?\\{}", path)
}

fn run_command(
    program: &str,
    arguments: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
) -> anyhow::Result<()> {
    let mut command = Command::new(program);
    command.args(arguments);
    let output = command
        .output()
        .context(format!("Calling {program} failed. Command:\n{command:?}"))?;
    if !output.status.success() {
        bail!(
            "{program} exited non-successfully.\n\nCommand:\n{command:?}\n\nStdout:\n{}\n\nStderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    Ok(())
}

fn build_admin_sid() -> anyhow::Result<PSID> {
    let mut admin_sid: PSID = PSID(null_mut());
    unsafe {
        AllocateAndInitializeSid(
            &SECURITY_NT_AUTHORITY,
            2,
            SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut admin_sid,
        )?;
    }
    Ok(admin_sid)
}

fn set_owner(path: &Utf8Path, owner_sid: PSID) -> anyhow::Result<()> {
    let wide_path: Vec<u16> = make_long_path(path)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let result = SetNamedSecurityInfoW(
            PCWSTR(wide_path.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(owner_sid),
            None,
            None,
            None,
        );
        if result.is_err() {
            bail!(
                "Failed to set owner on {path}: {result:?}",
                path = path,
                result = result
            );
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_long_path() {
        assert_eq!(
            make_long_path(Utf8Path::new(r"C:\some\normal\path")),
            r"\\?\C:\some\normal\path"
        );
    }

    #[ignore]
    #[test]
    fn test_take_ownership_recursively() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir_path: Utf8PathBuf = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;

        let test_dir = temp_dir_path.join("test_dir");
        std::fs::create_dir(&test_dir)?;
        let test_file = test_dir.join("test_file.txt");
        std::fs::write(&test_file, "")?;
        let sub_dir = test_dir.join("sub_dir");
        std::fs::create_dir(&sub_dir)?;
        let sub_file = sub_dir.join("sub_file.txt");
        std::fs::write(&sub_file, "")?;

        let current_user_name = std::env::var("USERNAME")?;
        // Transfer ownership to current user.
        // We could use this method as well in transfer_directory_ownership_to_admin_group_recursive,
        // but we want to move away from subprocesses where possible.
        run_icacls_command(&test_dir, ["/setowner", &current_user_name, "/T"])?;
        assert!(get_owner(&test_dir)?.ends_with(&current_user_name));
        assert!(get_owner(&test_file)?.ends_with(&current_user_name));
        assert!(get_owner(&sub_dir)?.ends_with(&current_user_name));
        assert!(get_owner(&sub_file)?.ends_with(&current_user_name));

        transfer_directory_ownership_to_admin_group_recursive(&test_dir)?;
        assert_eq!(get_owner(&test_dir)?, "BUILTIN\\Administrators");
        assert_eq!(get_owner(&test_file)?, "BUILTIN\\Administrators");
        assert_eq!(get_owner(&sub_dir)?, "BUILTIN\\Administrators");
        assert_eq!(get_owner(&sub_file)?, "BUILTIN\\Administrators");

        Ok(())
    }

    pub fn get_owner(path: &Utf8Path) -> anyhow::Result<String> {
        let script = format!("(Get-Acl -LiteralPath '{path}').Owner", path = path);
        let output = Command::new(
            "pwsh.exe", // Note: Only "pwsh.exe" works in GitHub Actions
        )
        .args(["-Command", &script])
        .output()?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        bail!(format!(
            "Failed to get owner: {stderr}",
            stderr = String::from_utf8_lossy(&output.stderr)
        ))
    }
}
