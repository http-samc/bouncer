use crate::policy::routes::RouteRegistration;
use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::Request,
    routing::get,
};

// Re-export the config type from v1
pub use super::v1::BearerAuthConfig;

// Policy implementation that inherits from v1 but with managed routes
pub struct BearerAuthManagedPolicy {
    inner: super::v1::BearerAuthPolicy,
}

// Policy factory for creating managed bearer auth policies
pub struct BearerAuthManagedPolicyFactory;

#[async_trait]
impl PolicyFactory for BearerAuthManagedPolicyFactory {
    type PolicyType = BearerAuthManagedPolicy;
    type Config = BearerAuthConfig;

    fn policy_id() -> &'static str {
        // Use the same provider ID as v1 but with -managed suffix
        crate::policy::providers::bouncer::auth::bearer::policy_id_with_version("v1-managed")
    }

    fn version() -> Option<&'static str> {
        Some("v1-managed")
    }

    async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // Create the inner v1 policy
        let inner = super::v1::BearerAuthPolicyFactory::new(config).await?;
        Ok(BearerAuthManagedPolicy { inner })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        super::v1::BearerAuthPolicyFactory::validate_config(config)
    }
}

#[async_trait]
impl Policy for BearerAuthManagedPolicy {
    fn provider(&self) -> &'static str {
        self.inner.provider()
    }

    fn category(&self) -> &'static str {
        self.inner.category()
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn version(&self) -> &'static str {
        "v1-managed"
    }

    fn register_routes(&self) -> Vec<RouteRegistration> {
        tracing::debug!("Registering routes for bearer auth policy v1-managed");
        vec![
            RouteRegistration {
                relative_path: "".to_string(), // Base path
                handler: get(|| async {
                    tracing::debug!("Bearer auth policy v1-managed handler called");
                    "Hello from Bearer Auth Policy v1-managed!"
                }),
            }
        ]
    }

    async fn process(&self, request: Request<Body>) -> PolicyResult {
        // Delegate to the inner v1 policy's process method
        self.inner.process(request).await
    }
}