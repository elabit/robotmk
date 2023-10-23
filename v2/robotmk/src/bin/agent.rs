use camino::Utf8PathBuf;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env::{var, VarError};
use std::fs::read_to_string;
use std::path::Path;
use walkdir::{DirEntry, Error, WalkDir};

#[derive(Deserialize)]
pub struct Config {
    pub results_directory: Utf8PathBuf,
}

#[derive(Parser)]
#[command(about = "Robotmk agent plugin.")]
struct Args {
    /// Configuration file path.
    #[clap(name = "CONFIG_PATH")]
    pub config_path: Option<Utf8PathBuf>,
}

#[derive(Serialize)]
pub struct ConfigError {
    config_reading_error: String,
}

#[derive(Serialize)]
pub struct ConfigFileContent {
    config_file_content: String,
}

fn determine_config_path(arg: Option<Utf8PathBuf>) -> Result<Utf8PathBuf, String> {
    if let Some(p) = arg {
        return Ok(p);
    }
    let config_dir = match var("MK_CONFDIR") {
        Ok(path) => path,
        Err(VarError::NotPresent) => "C:\\ProgramData\\checkmk\\agent\\config".to_string(),
        Err(VarError::NotUnicode(_path)) => return Err("CONFIG_PATH is not utf-8.".into()),
    };
    let mut config_dir = Utf8PathBuf::from(config_dir);
    config_dir.push("robotmk.json");
    Ok(config_dir)
}

fn report_config_error(message: String) {
    let config_error = serde_json::to_string(&ConfigError {
        config_reading_error: message,
    })
    .unwrap();
    println!("{config_error}");
}

fn report_config_content(content: String) {
    let config_content = serde_json::to_string(&ConfigFileContent {
        config_file_content: content,
    })
    .unwrap();
    println!("{config_content}");
}

fn print_or_ignore(entry: Result<DirEntry, Error>) {
    if let Ok(dir) = entry {
        if dir.file_type().is_file() {
            if let Ok(raw) = read_to_string(dir.path()) {
                println!("{raw}");
            }
        }
    }
}

fn walk(results_directory: &Path) {
    for entry in WalkDir::new(results_directory)
        .sort_by_file_name()
        .into_iter()
    {
        print_or_ignore(entry);
    }
}

fn main() {
    let arguments = Args::parse();
    println!("<<<robotmk_v2:sep(10)>>>");
    let config_path = match determine_config_path(arguments.config_path) {
        Ok(p) => p,
        Err(e) => {
            report_config_error(e);
            return;
        }
    };
    let raw = match read_to_string(config_path) {
        Ok(raw) => raw,
        Err(e) => {
            report_config_error(e.to_string());
            return;
        }
    };
    report_config_content(raw.clone());
    let config: Config = match serde_json::from_str(&raw) {
        Ok(config) => config,
        Err(e) => {
            report_config_error(e.to_string());
            return;
        }
    };
    walk(&config.results_directory.into_std_path_buf());
}
