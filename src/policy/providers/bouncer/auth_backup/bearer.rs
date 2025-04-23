// This file is kept for backward compatibility
// It re-exports the default version of the bearer policy
// We're using a module with the same name but different structure

// Re-export everything from the default version
pub use crate::policy::providers::bouncer::auth::bearer::v1::*;

// This struct is a factory that returns the default version of the BearerAuthPolicy
use crate::policy::traits::{Policy, PolicyFactory};
use async_trait::async_trait;

pub struct BearerAuthPolicyFactory;

#[async_trait]
impl PolicyFactory for BearerAuthPolicyFactory {
    type PolicyType = BearerAuthPolicy;
    type Config = BearerAuthConfig;

    fn policy_id() -> &'static str {
        // For backward compatibility, this returns the non-versioned policy ID
        "@bouncer/auth/bearer"
    }

    async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // Delegate to the default version's implementation
        v1::BearerAuthPolicyFactory::new(config).await
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        // Delegate to the default version's implementation
        v1::BearerAuthPolicyFactory::validate_config(config)
    }
}
