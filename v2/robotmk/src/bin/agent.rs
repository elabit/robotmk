use camino::Utf8PathBuf;
use clap::Parser;
use robotmk::config::Config;
use serde::Serialize;
use std::env::{var, VarError};
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

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

#[derive(Deserialize)]
pub struct Section {
    pub name: String,
    pub content: String,
}

fn determine_config_path(arg: Option<Utf8PathBuf>) -> Result<Utf8PathBuf, String> {
    Ok(arg.unwrap_or(config_path_from_env()?))
}

fn config_path_from_env() -> Result<Utf8PathBuf, String> {
    let config_path = match var("MK_CONFDIR") {
        Ok(path) => path,
        Err(VarError::NotPresent) => "C:\\ProgramData\\checkmk\\agent\\config".into(),
        Err(VarError::NotUnicode(_path)) => return Err("CONFIG_PATH is not utf-8.".into()),
    };
    Ok(Utf8PathBuf::from(config_path).join("robotmk.json"))
}

fn report_config_error(message: String) {
    let config_error = serde_json::to_string(&ConfigError {
        config_reading_error: message,
    })
    .expect("Unexpected serialization error: ConfigError");
    println!("{config_error}");
}

fn report_config_content(content: String) {
    let config_content = serde_json::to_string(&ConfigFileContent {
        config_file_content: content,
    })
    .expect("Unexpected serialization error: ConfigFileContent");
    println!("{config_content}");
}

pub fn read(directory: impl AsRef<Path>) -> Vec<Section> {
    // TODO: Test this function.
    let mut sections = Vec::new();
    for entry in WalkDir::new(directory)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(raw) = read_to_string(entry.path()) {
                let section: Result<Section, _> = serde_json::from_str(&raw);
                if let Ok(section) = section {
                    sections.push(section)
                }
            }
        }
    }
    sections
}

fn print_sections(sections: &[Section], stdout: &mut impl io::Write) {
    // TODO: Test this function.
    for section in sections.iter() {
        let with_header = format!("<<<{}>>>\n{}\n", section.name, section.content);
        write!(stdout, "{}", with_header).unwrap();
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
    let sections = read(config.results_directory);
    print_sections(&sections, &mut io::stdout());
}
