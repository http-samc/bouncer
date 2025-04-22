use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Deserialize)]
pub struct PolicyConfig {
    pub id: String,
    pub provider: String,
    pub parameters: serde_json::Value,
}

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub policies: Vec<PolicyConfig>,
    // This will catch all other fields that don't match the above
    #[serde(flatten)]
    pub policy_configs: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Optional destination address to forward requests to after middleware processing.
    /// Can be a full URL like "http://api.example.com" or a local address like "http://localhost:3000"
    #[serde(default)]
    pub destination_address: Option<String>,
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Config {
    // Generate policy configs from the flattened map
    pub fn process_policy_configs(&mut self) {
        for (key, value) in self.policy_configs.iter() {
            // Skip entries that don't look like policy identifiers
            if !key.starts_with('@') {
                continue;
            }

            self.policies.push(PolicyConfig {
                id: key.clone(),
                provider: key.clone(), // The provider is the same as the key in this new format
                parameters: value.clone(),
            });
        }
    }

    // Construct the bind address string with port
    pub fn full_bind_address(&self) -> String {
        format!("{}:{}", self.server.bind_address, self.server.port)
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut config: Config =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    // Process the policy configs to generate the policies array
    config.process_policy_configs();

    Ok(config)
}
