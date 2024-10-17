use crate::internal_config::{Plan, Source};
use anyhow::Context;
use camino::Utf8Path;
use flate2::read::GzDecoder;
use log::{error, info};
use robotmk::results::SetupFailure;
use std::fs::File;
use tar::Archive;

const SIZE_LIMIT: u64 = 50 * 1024 * 1024;

pub fn setup(plans: Vec<Plan>) -> (Vec<Plan>, Vec<SetupFailure>) {
    let mut surviving_plans = Vec::new();
    let mut failures = vec![];
    for plan in plans.into_iter() {
        if let Source::Managed {
            tar_gz_path,
            target,
            ..
        } = &plan.source
        {
            if let Err(error) = unpack_into(tar_gz_path, target, SIZE_LIMIT) {
                error!(
                    "Plan {}: Failed to unpack managed source archive. Plan won't be scheduled.
                     Error: {error:?}",
                    plan.id
                );
                failures.push(SetupFailure {
                    plan_id: plan.id.clone(),
                    summary: "Failed to unpack managed source archive".to_string(),
                    details: format!("{error:?}"),
                });
                continue;
            }
            info!("Unpacked {} into `{}`.", tar_gz_path, target);
        }
        surviving_plans.push(plan);
    }
    (surviving_plans, failures)
}

fn unpack_into(
    tar_gz_path: &Utf8Path,
    target_path: &Utf8Path,
    size_limit: u64,
) -> anyhow::Result<()> {
    info!("Extracting archive \"{tar_gz_path}\"");
    // We have to open the archive twice. Re-using the already opened archive for extraction does
    // not work.
    let archive_size = sum_up_size_of_archive_entries(&mut open_tar_gz_archive(tar_gz_path)?)
        .context("Failed to compute archive size")?;
    if archive_size > size_limit {
        anyhow::bail!(format!(
            "Archive size exceeds limit: {archive_size} B > {size_limit} B"
        ))
    }
    open_tar_gz_archive(tar_gz_path)?.unpack(target_path)?;
    Ok(())
}

fn open_tar_gz_archive(path: &Utf8Path) -> std::io::Result<Archive<GzDecoder<File>>> {
    let tar_gz = File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    Ok(Archive::new(tar))
}

fn sum_up_size_of_archive_entries<R: Sized + std::io::Read>(
    archive: &mut Archive<R>,
) -> anyhow::Result<u64> {
    let mut sum = 0;
    for entry in archive.entries()? {
        let entry_size = entry?.size();
        // protect against attempts to fake the size
        if entry_size > 0 {
            sum += entry_size;
        }
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs;
    use std::io::{self, Write};
    use tempfile::tempdir;

    #[test]
    fn unpack_into_ok() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let temp_dir_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;

        let dir_to_be_archived = temp_dir_path.join("archive");
        fs::create_dir(&dir_to_be_archived)?;
        let mut file_in_archive = fs::File::create(dir_to_be_archived.join("file.txt"))?;
        file_in_archive.write_all(b"123abc")?;

        let archive_path = temp_dir_path.join("archive.tar.gz");
        archive_directory(&dir_to_be_archived, &archive_path, "archived")?;
        unpack_into(&archive_path, &temp_dir_path, 1024)?;

        assert_eq!(
            String::from_utf8(fs::read(temp_dir_path.join("archived").join("file.txt"))?)?,
            "123abc"
        );
        Ok(())
    }

    #[test]
    fn unpack_into_size_limit_exceeded() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let temp_dir_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;

        let dir_to_be_archived = temp_dir_path.join("archive");
        fs::create_dir(&dir_to_be_archived)?;
        let mut file_in_archive = fs::File::create(dir_to_be_archived.join("file.txt"))?;
        file_in_archive.write_all(b"123abc")?;

        let archive_path = temp_dir_path.join("archive.tar.gz");
        archive_directory(&dir_to_be_archived, &archive_path, "archived")?;
        let error = unpack_into(&archive_path, &temp_dir_path, 1).unwrap_err();
        assert!(format!("{error:?}").contains("Archive size exceeds limit: 6 B > 1 B"));

        Ok(())
    }

    fn archive_directory(
        dir_to_be_archived: &Utf8Path,
        archive_path: &Utf8Path,
        archived_name: &str,
    ) -> io::Result<()> {
        let mut archive_builder = tar::Builder::new(GzEncoder::new(
            File::create(archive_path)?,
            Compression::default(),
        ));
        archive_builder.append_dir_all(archived_name, dir_to_be_archived)?;
        Ok(())
    }
}
