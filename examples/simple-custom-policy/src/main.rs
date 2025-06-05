use async_trait::async_trait;
use axum::http::Request;
use bouncer::{Policy, PolicyFactory, PolicyResult, start_with_config, register_custom_policy};
use serde::Deserialize;

// Configuration for our custom rate limiting policy
#[derive(Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst: u32,
}

// Our custom policy implementation
pub struct RateLimitPolicy {
    config: RateLimitConfig,
}

// Factory for creating our policy
pub struct RateLimitPolicyFactory;

impl PolicyFactory for RateLimitPolicyFactory {
    type PolicyType = RateLimitPolicy;
    type Config = RateLimitConfig;

    fn policy_id() -> &'static str {
        "rate-limiter"
    }

    fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        Ok(RateLimitPolicy { config })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        if config.requests_per_minute == 0 {
            return Err("requests_per_minute must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[async_trait]
impl Policy for RateLimitPolicy {
    async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult {
        // In a real implementation, we would check if the client has exceeded
        // their rate limit and reject the request if necessary

        // This is just a placeholder implementation
        let client_ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");

        println!("Rate limiting request from {}", client_ip);
        println!("  Limit: {} requests per minute, burst: {}",
            self.config.requests_per_minute, self.config.burst);

        // Just continue for this example
        PolicyResult::Continue(request)
    }
}

// Register the policy in the main function instead
#[tokio::main]
async fn main() {
    // Register our custom policy
    register_custom_policy(|registry| {
        registry.register_policy::<RateLimitPolicyFactory>();
    });
    
    // Set up logging
    tracing_subscriber::fmt::init();
    
    println!("Starting Bouncer server with custom policy...");
    println!("Make HTTP requests to http://127.0.0.1:8080 to test the policy");
    
    // Start the Bouncer server with our config
    // This will automatically use our registered policy
    start_with_config("examples/simple-custom-policy/config.yaml").await;
} 