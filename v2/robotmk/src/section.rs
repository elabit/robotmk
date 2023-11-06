use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

#[derive(Deserialize, Serialize)]
pub struct Section {
    pub name: String,
    pub content: String,
}

fn write(name: String, content: &impl Serialize, path: impl AsRef<Utf8Path>) -> Result<()> {
    let path = path.as_ref();
    let content = serde_json::to_string(content).unwrap();
    let section = Section { name, content };
    let section = serde_json::to_string(&section).unwrap();
    let mut file = NamedTempFile::new().context("Opening tempfile failed")?;
    file.write_all(section.as_bytes()).context(format!(
        "Writing tempfile failed, {}",
        file.path().display()
    ))?;
    file.persist(path)
        .context(format!("Persisting tempfile failed, final_path: {path}"))
        .map(|_| ())
}

pub trait WriteSection {
    fn name() -> &'static str;

    fn write(&self, path: impl AsRef<Utf8Path>) -> Result<()>
    where
        Self: Serialize,
    {
        write(Self::name().into(), &self, path)
    }
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
