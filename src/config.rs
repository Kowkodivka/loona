use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub database: Database,
    pub discord: Discord,
}

impl Config {
    pub async fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Discord {
    pub token: String,
}
