use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub canvas_url: String,
    pub api_token: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                let contents = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config at {}", path.display()))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| "Failed to parse config.toml")?;
                return Ok(config);
            }
        }

        let canvas_url = std::env::var("CANVAS_URL")
            .with_context(|| "CANVAS_URL not set. Create a config file or set the env var.")?;
        let api_token = std::env::var("CANVAS_API_TOKEN")
            .with_context(|| "CANVAS_API_TOKEN not set. Create a config file or set the env var.")?;

        Ok(Self {
            canvas_url,
            api_token,
        })
    }

    pub fn generate_default() -> Result<PathBuf> {
        let path = Self::config_path()
            .with_context(|| "Could not determine config directory")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default = Config {
            canvas_url: "https://your-school.instructure.com".into(),
            api_token: "your-api-token-here".into(),
        };

        let toml_str = toml::to_string_pretty(&default)?;
        std::fs::write(&path, toml_str)?;
        Ok(path)
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("canvas-tui").join("config.toml"))
    }
}
