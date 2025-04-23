use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Redis,
    Postgres,
    Mysql,
    Mongo,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RedisConfig {
    pub connection_url: String,
    pub password: Option<String>,
    pub database: Option<u16>,
    pub timeout: Option<u64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PostgresConfig {
    pub connection_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub connection_pool_size: Option<u32>,
    pub ssl: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MySqlConfig {
    pub connection_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub connection_pool_size: Option<u32>,
    pub ssl: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MongoConfig {
    pub connection_uri: String,
    pub database: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DatabasesConfig {
    pub redis: Option<RedisConfig>,
    pub postgres: Option<PostgresConfig>,
    pub mysql: Option<MySqlConfig>,
    pub mongo: Option<MongoConfig>,
}

#[derive(Deserialize, Clone)]
pub struct PolicyConfig {
    pub id: String,
    pub provider: String,
    pub parameters: serde_json::Value,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub policies: Vec<PolicyConfig>,
    #[serde(default)]
    pub databases: DatabasesConfig,
    // This will catch all other fields that don't match the above
    #[serde(flatten)]
    pub policy_configs: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Clone)]
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
