#[cfg(unix)]
use anyhow::Context;
use camino::Utf8Path;
#[cfg(unix)]
use camino::Utf8PathBuf;

#[cfg(unix)]
pub fn transfer_directory_ownership_recursive(target: &Utf8Path) -> anyhow::Result<()> {
    let user_id = unsafe { libc::getuid() };
    let group_id = unsafe { libc::getgid() };
    let mut targets: Vec<Utf8PathBuf> = vec![target.into()];
    while let Some(target) = targets.pop() {
        std::os::unix::fs::lchown(&target, Some(user_id), Some(group_id)).context(format!(
            "Failed to set ownership of {target} to `{user_id}:{group_id}`",
        ))?;
        if target.is_dir() && !target.is_symlink() {
            targets.extend(super::fs_entries::top_level_directory_entries(&target)?);
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn transfer_directory_ownership_recursive(target: &Utf8Path) -> anyhow::Result<()> {
    super::windows_permissions::transfer_directory_ownership_to_admin_group_recursive(target)
}
