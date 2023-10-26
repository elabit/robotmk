use camino::Utf8PathBuf;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env::{var, VarError};
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use strum_macros::IntoStaticStr;
use walkdir::{DirEntry, WalkDir};

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
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
enum Section {
    ConfigError { error: Option<String> },
    ConfigFileContent { content: Option<String> },
}

impl Section {
    fn section_header(&self) -> &'static str {
        self.into()
    }

    fn section(&self) -> String
    where
        Self: Serialize,
    {
        let header = self.section_header();
        format!(
            "<<<robotmk_{}>>>\n{}",
            header,
            serde_json::to_string(&self)
                .unwrap_or_else(|_| panic!("Unexpected serialization error: {header}")),
        )
    }
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

fn print_or_ignore(dir: DirEntry, stdout: &mut impl io::Write) {
    if dir.file_type().is_file() {
        if let Ok(raw) = read_to_string(dir.path()) {
            writeln!(stdout, "{raw}").unwrap();
        }
    }
}

fn walk(results_directory: &impl AsRef<Path>, stdout: &mut impl io::Write) {
    for entry in WalkDir::new(results_directory)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        print_or_ignore(entry, stdout);
    }
}

fn read_config(config_path: Option<Utf8PathBuf>) -> Result<String, String> {
    let config_path = determine_config_path(config_path)?;
    read_to_string(config_path).map_err(|e| e.to_string())
}

fn parse_config(config_raw: &Result<String, String>) -> Result<Config, String> {
    let config_raw: &str = config_raw.as_deref()?;
    serde_json::from_str(config_raw).map_err(|e| e.to_string())
}

fn main() {
    let arguments = Args::parse();
    let config_raw = read_config(arguments.config_path);
    let config = parse_config(&config_raw);

    let content = config_raw.ok();
    let config_content = Section::ConfigFileContent { content };
    println!("{}", config_content.section());

    println!("<<<robotmk_v2:sep(10)>>>");
    if let Ok(config) = &config {
        walk(&config.results_directory, &mut io::stdout());
    }

    let error = config.err();
    let config_error = Section::ConfigError { error };
    println!("{}", config_error.section());
}

#[test]
fn test_walk() {
    use std::fs::{create_dir_all, File};
    use std::io::Write;
    use std::str::from_utf8_unchecked;
    use tempfile::tempdir;
    // Assemble
    let path_content = [
        ("1", "Failure is not an Option<T>, it's a Result<T,E>."),
        ("2", "In Rust, None is always an Option<_>."),
        ("3/nested", "Rust is the best thing since &Bread[..]."),
        ("4/more/nesting", "Yes, I stole these jokes from reddit."),
    ];
    let expected: String = path_content.map(|(_, c)| format!("{c}\n")).concat();
    let results_directory = tempdir().unwrap();
    for (path, content) in path_content {
        let file_path = results_directory.path().join(path);
        create_dir_all(file_path.parent().unwrap()).unwrap();
        let mut file = File::create(file_path).unwrap();
        write!(file, "{}", content).unwrap();
    }
    let mut stdout = Vec::new();
    //Act
    walk(&results_directory, &mut stdout);
    //Assert
    assert_eq!(unsafe { from_utf8_unchecked(&stdout) }, expected);
}
