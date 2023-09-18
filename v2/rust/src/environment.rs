use std::path::{Path, PathBuf};

pub fn environment_building_stdio_directory(working_directory: &Path) -> PathBuf {
    working_directory.join("environment_building_stdio")
}
