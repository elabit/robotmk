use clap::Parser;
use std::path::PathBuf;
use std::env;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

enum Errors {
    Env(env::VarError),
    Open(io::Error),
}

#[derive(Serialize)]
struct ConfigReadingError {
    config_reading_error: String,
}

#[derive(Serialize)]
struct ConfigContent {
    config_file_content: String,
}

#[derive(Deserialize)]
struct Config{
    results_directory: String,
}

#[derive(Parser)]
#[command(about = "Robotmk collector.")]
pub struct Args {
    /// Configuration file path.
    #[arg(name = "CONFIG_PATH")]
    pub config_path: Option<PathBuf>,
}

fn default_directory() -> PathBuf {
    let mk_confdir = env::var("MK_CONFDIR");
    if let Ok(mk_confdir) = mk_confdir {
        return PathBuf::from(mk_confdir).join("robotmk.json")
    }
    panic!("Path to configuration is needed!");
}

fn main() {
    println!("<<<robotmk_v2:sep(10)>>>");
    let args = Args::parse();
    let config_path = args.config_path.unwrap_or_else(default_directory);
    let config_content = fs::read_to_string(config_path);
    if let Err(error) = config_content {
        let error = ConfigReadingError{config_reading_error: format!("{:?}", error)};
        println!("{}", serde_json::to_string(&error).unwrap());
    } else {
        let config_content = config_content.unwrap();
        let content = ConfigContent{config_file_content: format!("{:?}", config_content)};
        println!("{}", serde_json::to_string(&content).unwrap());
        let config: Result<Config, _> = serde_json::from_str(&config_content);
        let config = config.unwrap();
        let paths = fs::read_dir(config.results_directory).unwrap();
        for path in paths {
            let content = fs::read_to_string(path.unwrap().path()).unwrap();
            println!("{}", content)
        }
    }
}
