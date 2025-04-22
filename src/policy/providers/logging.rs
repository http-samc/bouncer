use axum::http::Request;
use serde::Deserialize;
use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;

// Configuration for the logging policy
#[derive(Deserialize)]
pub struct LoggingConfig {
    pub log_level: String,
    pub include_headers: bool,
}

// Simple policy for logging requests
pub struct LoggingPolicy {
    config: LoggingConfig,
}

// Factory implementation
impl PolicyFactory for LoggingPolicy {
    type PolicyType = Self;
    type Config = LoggingConfig;

    fn policy_id() -> &'static str {
        "logging"
    }

    fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        Ok(LoggingPolicy { config })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        let valid_levels = ["debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.log_level.to_lowercase().as_str()) {
            return Err(format!("Invalid log level: {}. Must be one of: {:?}", 
                               config.log_level, valid_levels));
        }
        Ok(())
    }
}

// Policy implementation
#[async_trait]
impl Policy for LoggingPolicy {
    async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult {
        // Extract request info
        let method = request.method().clone();
        let uri = request.uri().clone();
        
        // Log the request based on configured level
        match self.config.log_level.to_lowercase().as_str() {
            "debug" => {
                if self.config.include_headers {
                    let headers = request.headers().clone();
                    tracing::debug!("Request: {} {} with headers: {:?}", method, uri, headers);
                } else {
                    tracing::debug!("Request: {} {}", method, uri);
                }
            },
            "info" => tracing::info!("Request: {} {}", method, uri),
            "warn" => tracing::warn!("Request: {} {}", method, uri),
            "error" => tracing::error!("Request: {} {}", method, uri),
            _ => {}
        }
        
        // Always continue with the original request
        PolicyResult::Continue(request)
    }
} 