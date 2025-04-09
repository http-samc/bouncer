use axum::{Router};
use axum_server::Server;
use crate::policy::PolicyRegistry;

pub async fn start_server(config: crate::config::Config) {
    let registry = PolicyRegistry::new();

    // Build policy chain based on config file
    let policy_chain = registry.build_policy_chain(&config.policies)
        .expect("Failed to build policy chain");

    // Create Axum router with middleware for policies
    let app = Router::new()
        .route("/*path", axum::routing::any(handler))
        .layer(policy_chain.into_middleware());

    // Start the HTTP server
    let addr = config.server.bind_address.parse().expect("Invalid bind address");
    tracing::info!("Starting server on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Server failed");
}

// Example handler for processing requests after middleware executes.
async fn handler() -> &'static str {
    "Hello from Bouncer!"
}
