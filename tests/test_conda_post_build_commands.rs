pub mod helper;
use crate::helper::var;
use camino::Utf8PathBuf;
use chrono::Utc;
use robotmk::config::{CondaEnvironmentSource, HTTPProxyConfig, TlsCertificateValidation};
use robotmk::env::{Environment, conda::CondaEnvironment};
use robotmk::results::BuildOutcome;
use robotmk::session::{CurrentSession, Session};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

const CONDA_MANIFEST_FILENAME: &str = "empty_env.yaml";

#[cfg(unix)]
const RMK_MANIFEST_FILENAME: &str = "rmk_env_manifest_linux.yaml";
#[cfg(unix)]
const PATH_POST_BUILD_MARKER: &str = "/tmp/robotmk_post_build_command_test";
#[cfg(windows)]
const RMK_MANIFEST_FILENAME: &str = "rmk_env_manifest_windows.yaml";
#[cfg(windows)]
const PATH_POST_BUILD_MARKER: &str = "C:\\robotmk_post_build_command_test";

#[test]
#[ignore]
fn test_conda_post_build_commands() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let temp_dir_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;
    let tests_dir_path = Utf8PathBuf::from(var("CARGO_MANIFEST_DIR")?).join("tests");

    let build_outcome = Environment::Conda(CondaEnvironment {
        source: CondaEnvironmentSource::Manifest(tests_dir_path.join(CONDA_MANIFEST_FILENAME)),
        micromamba_binary_path: Utf8PathBuf::from(var("MICROMAMBA_BINARY_PATH")?),
        robotmk_manifest_path: Some(tests_dir_path.join(RMK_MANIFEST_FILENAME)),
        root_prefix: temp_dir_path.join("mamba_root"),
        prefix: temp_dir_path.join("env"),
        http_proxy_config: HTTPProxyConfig::default(),
        tls_certificate_validation: TlsCertificateValidation::Enabled,
        tls_revoke_active: false,
        build_timeout: var("BUILD_TIMEOUT")?.parse::<u64>()?,
        build_runtime_directory: temp_dir_path,
    })
    .build(
        "test_conda_post_build_commands",
        &Session::Current(CurrentSession {}),
        Utc::now(),
        &CancellationToken::default(),
    )?;

    assert!(matches!(build_outcome, BuildOutcome::Success(_)));
    assert!(std::path::PathBuf::from(PATH_POST_BUILD_MARKER).is_file());

    Ok(())
}
