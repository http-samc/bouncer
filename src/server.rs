use axum::Router;
use axum_server::Server;
use crate::policy::registry::PolicyRegistry;
use crate::policy::PolicyChainExt;
use crate::policy::providers::bouncer::auth::bearer::BearerAuthPolicyFactory;
use std::net::SocketAddr;
use std::path::Path;

pub async fn start_server(config: crate::config::Config) {
    // Create policy registry and register all available policies
    let mut registry = PolicyRegistry::new();

    // Register built-in policies
    register_policies(&mut registry);

    // Load external policies from plugins directory if it exists
    let plugins_dir = Path::new("plugins");
    if plugins_dir.exists() && plugins_dir.is_dir() {
        match registry.load_policies_from_directory(plugins_dir) {
            Ok(_) => tracing::info!("Loaded external policies from plugins directory"),
            Err(e) => tracing::warn!("Failed to load external policies: {}", e),
        }
    }

    // Build policy chain based on config file
    let policy_chain = registry.build_policy_chain(&config.policies)
        .expect("Failed to build policy chain");

    // Create Axum router with middleware for policies
    let app = Router::new()
        .route("/*path", axum::routing::any(handler))
        .layer(policy_chain.into_layer());

    // Start the HTTP server
    let addr: SocketAddr = config.full_bind_address().parse().expect("Invalid bind address");

    tracing::info!("Starting server on {}", addr);

    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .expect("Server failed");
}

// Example handler for processing requests after middleware executes.
async fn handler() -> &'static str {
    "Hello from Bouncer!"
}

// Function to register all available policies
fn register_policies(registry: &mut PolicyRegistry) {
    registry.register_policy::<BearerAuthPolicyFactory>();
}
