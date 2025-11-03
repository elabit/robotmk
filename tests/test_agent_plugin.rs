use assert_cmd::cargo::cargo_bin_cmd;
use camino::{Utf8Path, Utf8PathBuf};
use robotmk::config::{CondaConfig, Config, RCCConfig, RCCProfileConfig};
use robotmk::lock::Locker;
use robotmk::results::{ConfigSection, results_directory};
use robotmk::section::{Host, WritePiggybackSection, WriteSection};
use serde::Serialize;
use std::fs::{create_dir, write};
use std::io;
use std::process::Output;
use tempfile::tempdir;

#[test]
#[ignore]
fn test_agent_plugin() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let temp_dir_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())?;

    let config = create_config(&temp_dir_path);
    let config_path = temp_dir_path.join("robotmk.json");
    write(&config_path, serde_json::to_string(&config)?)?;
    create_dir(results_directory(&temp_dir_path))?;
    write_results(&results_directory(&temp_dir_path), &config_path)?;

    let output = run_agent_plugin(&temp_dir_path)?;
    assert!(output.status.success());

    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!(
            "<<<robotmk_config_v2:sep(0)>>>\n{}\n{}",
            serde_json::to_string(&ConfigSection::FileContent(serde_json::to_string(&config)?))?,
            "<<<section:sep(0)>>>
{\"a\":\"a\",\"b\":123}
<<<<piggy>>>>
<<<piggyback_section:sep(0)>>>
{\"x\":true,\"y\":\"some-string\"}
<<<<>>>>
"
        )
    );
    assert!(output.stderr.is_empty());
    Ok(())
}

fn create_config(runtime_dir: &Utf8Path) -> Config {
    Config {
        runtime_directory: runtime_dir.into(),
        rcc_config: RCCConfig {
            binary_path: "".into(),
            profile_config: RCCProfileConfig::Default,
            robocorp_home_base: "".into(),
        },
        conda_config: CondaConfig {
            micromamba_binary_path: "/micromamba".into(),
            base_directory: Utf8PathBuf::default(),
        },
        plan_groups: vec![],
    }
}

fn write_results(results_dir: &Utf8Path, config_path: &Utf8Path) -> anyhow::Result<()> {
    let locker = Locker::new(config_path, None);
    let sub_dir = results_dir.join("sub");
    create_dir(&sub_dir)?;
    Section {
        a: "a".into(),
        b: 123,
    }
    .write(results_dir.join("section.json"), &locker)?;
    PiggybackSection {
        x: true,
        y: "some-string".into(),
    }
    .write(
        sub_dir.join("piggyback_section.json"),
        Host::Piggyback("piggy".into()),
        &locker,
    )?;
    Ok(())
}

#[derive(Serialize)]
struct Section {
    a: String,
    b: i64,
}

impl WriteSection for Section {
    fn name() -> &'static str {
        "section"
    }
}

#[derive(Serialize)]
struct PiggybackSection {
    x: bool,
    y: String,
}

impl WritePiggybackSection for PiggybackSection {
    fn name() -> &'static str {
        "piggyback_section"
    }
}

fn run_agent_plugin(config_dir: &Utf8Path) -> io::Result<Output> {
    let mut agent_plugin_cmd = cargo_bin_cmd!("robotmk_agent_plugin");
    agent_plugin_cmd.env("MK_CONFDIR", config_dir);
    agent_plugin_cmd.output()
}
