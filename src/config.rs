use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub database: Database,
    pub discord: Discord,
}

impl Config {
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub async fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    pub async fn load_or_init<P: AsRef<Path>>(path: P) -> anyhow::Result<Option<Self>> {
        let path = path.as_ref();

        if !path.try_exists()? {
            Self::default().save(path).await?;
            return Ok(None);
        }

        Ok(Some(Self::load(path).await?))
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
