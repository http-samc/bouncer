pub mod config;
pub mod policy;
pub mod server;

use once_cell::sync::Lazy;
use policy::registry::PolicyRegistry;
use std::sync::Mutex;

// Re-export key components for convenience
pub use policy::traits::{Policy, PolicyFactory, PolicyResult};

// Simplified API for library users
pub use server::start_server;

// Global registry for storing custom policy factories
static CUSTOM_POLICIES: Lazy<Mutex<Vec<fn(&mut PolicyRegistry)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

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
/// use bouncer::{register_custom_policy, PolicyFactory};
///
/// pub struct MyCustomPolicyFactory;
///
/// impl PolicyFactory for MyCustomPolicyFactory {
///     // Implementation details...
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
