use crate::Error;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a config path next to the running executable (fallback: current dir).
pub fn default_config_beside_exe(file_name: &str) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join(file_name);
        }
    }
    PathBuf::from(file_name)
}

/// Load and parse a TOML config file.
pub fn load_toml_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Error> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|e| {
        Error::config(format!("failed to read config {}: {}", path.display(), e))
    })?;
    toml::from_str(&content).map_err(|e| {
        Error::config(format!("failed to parse config {}: {}", path.display(), e))
    })
}

/// Write text to a path, creating parent directories as needed.
pub fn write_config_file(path: impl AsRef<Path>, content: &str) -> Result<(), Error> {
    let path = path.as_ref();
    ensure_parent_dir(path)?;
    fs::write(path, content).map_err(|e| {
        Error::config(format!("failed to write config {}: {}", path.display(), e))
    })
}

/// Ensure parent directory exists for a path.
pub fn ensure_parent_dir(path: impl AsRef<Path>) -> Result<(), Error> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::config(format!("failed to create dir {}: {}", parent.display(), e))
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::io::Write;

    #[derive(Debug, Deserialize)]
    struct Sample {
        name: String,
        port: u16,
    }

    #[test]
    fn load_toml_ok() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg.toml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "name = \"demo\"\nport = 7000").unwrap();
        let cfg: Sample = load_toml_config(&path).unwrap();
        assert_eq!(cfg.name, "demo");
        assert_eq!(cfg.port, 7000);
    }

    #[test]
    fn write_config_creates_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("c.toml");
        write_config_file(&path, "x = 1\n").unwrap();
        assert!(path.is_file());
    }
}
