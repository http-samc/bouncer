use serde::{Deserialize};
use std::{fs, path::Path};

#[derive(Deserialize)]
pub struct PolicyConfig {
    pub id: String,
    pub provider: String,
    pub parameters: serde_json::Value,
}

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub policies: Vec<PolicyConfig>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))
}
