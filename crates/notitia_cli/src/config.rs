use std::path::{Path, PathBuf};

use anyhow::bail;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub snapshots_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            snapshots_dir: PathBuf::from("snapshots"),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = Path::new("notitia.toml");
        if !path.exists() {
            bail!("notitia.toml not found. Run `notitia init` to create one.");
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;

        Ok(config)
    }
}
