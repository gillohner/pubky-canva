use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub watcher: WatcherConfig,
    pub canvas: CanvasConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub listen: String,
}

#[derive(Deserialize, Clone)]
pub struct WatcherConfig {
    pub poll_interval_ms: u64,
}

#[derive(Deserialize, Clone)]
pub struct CanvasConfig {
    pub initial_size: u32,
    pub max_credits: u32,
    pub credit_regen_seconds: u64,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
