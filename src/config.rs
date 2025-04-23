use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path, env};
use serde::de::{self, Deserializer, Visitor};
use std::fmt;

// Custom deserializer for strings that might contain environment variable references
fn deserialize_env_var<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringVisitor;

    impl<'de> Visitor<'de> for StringVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an environment variable reference")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if let Some(env_var) = value.strip_prefix("ENV.") {
                match env::var(env_var) {
                    Ok(val) => Ok(val),
                    Err(_) => {
                        // Return the original value if the environment variable isn't set
                        // This allows for fallback behavior
                        Ok(value.to_string())
                    }
                }
            } else {
                Ok(value.to_string())
            }
        }
    }

    deserializer.deserialize_str(StringVisitor)
}

// Custom deserializer for optional strings that might contain environment variable references
fn deserialize_optional_env_var<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // First deserialize to an Option<String>
    Option::<String>::deserialize(deserializer).map(|opt_string| {
        opt_string.map(|s| {
            // Process environment variables if present
            if let Some(env_var) = s.strip_prefix("ENV.") {
                env::var(env_var).unwrap_or(s)
            } else {
                s
            }
        })
    })
}

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
    #[serde(deserialize_with = "deserialize_env_var")]
    pub connection_url: String,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub password: Option<String>,
    pub database: Option<u16>,
    pub timeout: Option<u64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PostgresConfig {
    #[serde(deserialize_with = "deserialize_env_var")]
    pub connection_url: String,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub username: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub password: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub database: Option<String>,
    pub connection_pool_size: Option<u32>,
    pub ssl: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MySqlConfig {
    #[serde(deserialize_with = "deserialize_env_var")]
    pub connection_url: String,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub username: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub password: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_env_var", default)]
    pub database: Option<String>,
    pub connection_pool_size: Option<u32>,
    pub ssl: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MongoConfig {
    #[serde(deserialize_with = "deserialize_env_var")]
    pub connection_uri: String,
    #[serde(deserialize_with = "deserialize_env_var")]
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
    #[serde(deserialize_with = "deserialize_env_var")]
    pub bind_address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Optional destination address to forward requests to after middleware processing.
    /// Can be a full URL like "http://api.example.com" or a local address like "http://localhost:3000"
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_env_var")]
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

// Function to process environment variables in serde_json::Value
fn process_env_vars(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(env_var) = s.strip_prefix("ENV.") {
                if let Ok(val) = env::var(env_var) {
                    *s = val;
                }
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                process_env_vars(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                process_env_vars(v);
            }
        }
        _ => {}
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    
    // First parse to Value to allow processing environment variables
    let mut yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Process environment variables in the parsed YAML
    process_yaml_env_vars(&mut yaml_value);
    
    // Convert back to string and parse to our Config struct
    let yaml_str = serde_yaml::to_string(&yaml_value)
        .map_err(|e| format!("Failed to serialize processed YAML: {}", e))?;
    
    let mut config: Config = serde_yaml::from_str(&yaml_str)
        .map_err(|e| format!("Failed to parse YAML into Config: {}", e))?;

    // Process environment variables in policy configs
    for (_, value) in config.policy_configs.iter_mut() {
        process_env_vars(value);
    }
    
    // Process the policy configs to generate the policies array
    config.process_policy_configs();

    Ok(config)
}

// Process environment variables in YAML values
fn process_yaml_env_vars(value: &mut serde_yaml::Value) {
    match value {
        serde_yaml::Value::String(s) => {
            if let Some(env_var) = s.strip_prefix("ENV.") {
                if let Ok(val) = env::var(env_var) {
                    *s = val;
                }
            }
        }
        serde_yaml::Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                process_yaml_env_vars(v);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for v in seq.iter_mut() {
                process_yaml_env_vars(v);
            }
        }
        _ => {}
    }
}
