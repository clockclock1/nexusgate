use crate::Error;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

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
}
