pub mod config;
pub mod database;
pub mod policy;
pub mod server;

use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use policy::registry::PolicyRegistry;
use std::sync::Mutex;

// Re-export key components for convenience
pub use policy::traits::{Policy, PolicyFactory, PolicyResult};

// Simplified API for library users
pub use server::start_server;

// The crate version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Global registry for storing custom policy factories
static CUSTOM_POLICIES: Lazy<Mutex<Vec<fn(&mut PolicyRegistry)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

// Global configuration that can be accessed from anywhere in the code
pub static GLOBAL_CONFIG: OnceCell<config::Config> = OnceCell::new();

/// Convenience function to start a Bouncer server with the given config and custom policies
///
/// This provides a simple way to start a Bouncer server directly from your application.
///
/// # Example
///
/// ```rust,no_run
/// use bouncer::{start_with_config, register_policy};
///
/// #[tokio::main]
/// async fn main() {
///     // Start the server with a config file
///     bouncer::start_with_config("config.yaml").await;
/// }
/// ```
pub async fn start_with_config(config_path: &str) {
    // Load configuration file
    let config = match config::load_config(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Check version compatibility
    if let Err(e) = config::validate_version(&config.bouncer_version, VERSION) {
        eprintln!("Version compatibility error: {}", e);
        eprintln!(
            "Config version: {}, Bouncer version: {}",
            config.bouncer_version, VERSION
        );
        eprintln!("Hint: Update your config file with a compatible 'bouncer_version' field.");
        std::process::exit(1);
    }

    // Start the server with loaded configuration
    server::start_server(config).await;
}

/// Register a custom policy for use with Bouncer
///
/// This function allows registering custom policies without having to
/// use the plugin system. Policies registered this way will be available
/// when starting the server with `start_with_config`.
///
/// # Example
///
/// ```rust,no_run
/// use bouncer::{register_custom_policy, policy::traits::{Policy, PolicyFactory, PolicyResult}};
/// use async_trait::async_trait;
/// use axum::body::Body;
/// use axum::http::Request;
///
/// pub struct MyCustomPolicy;
///
/// #[async_trait]
/// impl Policy for MyCustomPolicy {
///     async fn process(&self, request: Request<Body>) -> PolicyResult {
///         // Implementation details...
///         PolicyResult::Continue(request)
///     }
/// }
///
/// pub struct MyCustomPolicyFactory;
///
/// #[async_trait]
/// impl PolicyFactory for MyCustomPolicyFactory {
///     type PolicyType = MyCustomPolicy;
///     type Config = serde_json::Value;
///
///     fn policy_id() -> &'static str {
///         "@mycustom/policy"
///     }
///
///     async fn new(_config: Self::Config) -> Result<Self::PolicyType, String> {
///         Ok(MyCustomPolicy)
///     }
///
///     fn validate_config(_config: &Self::Config) -> Result<(), String> {
///         Ok(())
///     }
/// }
///
/// fn main() {
///     register_custom_policy(|registry| {
///         registry.register_policy::<MyCustomPolicyFactory>();
///     });
/// }
/// ```
pub fn register_custom_policy(register_fn: fn(&mut PolicyRegistry)) {
    let mut policies = CUSTOM_POLICIES.lock().unwrap();
    policies.push(register_fn);
}

/// Get all registered policies
pub(crate) fn get_custom_policies() -> Vec<fn(&mut PolicyRegistry)> {
    let policies = CUSTOM_POLICIES.lock().unwrap();
    policies.clone()
}
