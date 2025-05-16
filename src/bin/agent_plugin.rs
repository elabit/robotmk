use camino::Utf8PathBuf;
use clap::Parser;
use robotmk::{
    config::Config,
    lock::Locker,
    results::ConfigSection,
    results::results_directory,
    section::{Host, Section, read},
};
use std::env::{VarError, var};
use std::fs::read_to_string;
use std::io;

#[derive(Parser)]
#[command(about = "Robotmk agent plugin.", version)]
struct Args {
    /// Configuration file path.
    #[clap(name = "CONFIG_PATH")]
    pub config_path: Option<Utf8PathBuf>,
}

fn determine_config_path(arg: Option<Utf8PathBuf>) -> Result<Utf8PathBuf, String> {
    Ok(arg.unwrap_or(config_path_from_env()?))
}

fn config_path_from_env() -> Result<Utf8PathBuf, String> {
    let config_dir = match var("MK_CONFDIR") {
        Ok(path) => path,
        #[cfg(unix)]
        Err(VarError::NotPresent) => "/etc/check_mk".into(),
        #[cfg(windows)]
        Err(VarError::NotPresent) => "C:\\ProgramData\\checkmk\\agent\\config".into(),
        Err(VarError::NotUnicode(path)) => return Err(format!("Path {path:?} is not utf-8.")),
    };
    Ok(Utf8PathBuf::from(config_dir).join("robotmk.json"))
}

fn report_config_section(section: &ConfigSection) {
    let section_serialized =
        serde_json::to_string(section).expect("Unexpected serialization error: ConfigSection");
    println!("<<<robotmk_config_v2:sep(0)>>>");
    println!("{section_serialized}");
}

fn print_sections(sections: &[Section], stdout: &mut impl io::Write) {
    // TODO: Test this function.
    for section in sections.iter() {
        let mut with_header = format!("<<<{}:sep(0)>>>\n{}\n", section.name, section.content);
        if let Host::Piggyback(host) = &section.host {
            with_header = format!("<<<<{}>>>>\n{}<<<<>>>>\n", host, with_header);
        }
        write!(stdout, "{}", with_header).unwrap();
    }
}

fn main() {
    let arguments = Args::parse();
    let config_path = match determine_config_path(arguments.config_path) {
        Ok(p) => p,
        Err(e) => {
            report_config_section(&ConfigSection::ReadingError(e));
            return;
        }
    };
    let raw = match read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(e) => {
            let message = format!("Error while reading {config_path}: {e}");
            report_config_section(&ConfigSection::ReadingError(message));
            return;
        }
    };
    let config: Config = match serde_json::from_str(&raw) {
        Ok(config) => config,
        Err(e) => {
            let message = format!("Error while reading {config_path}: {e}");
            report_config_section(&ConfigSection::ReadingError(message));
            return;
        }
    };
    report_config_section(&ConfigSection::FileContent(raw));
    let sections = read(
        results_directory(&config.runtime_directory),
        &Locker::new(&config_path, None),
    )
    .unwrap();
    print_sections(&sections, &mut io::stdout());
}
