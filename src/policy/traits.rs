use async_trait::async_trait;
use axum::http::{Request, Response};
use serde::Deserialize;

pub enum PolicyResult {
    Continue(Request<axum::body::Body>),
    Terminate(Response<axum::body::Body>),
}

#[async_trait]
pub trait PolicyFactory {
    type PolicyType: Policy;
    type Config: for<'de> Deserialize<'de> + Send + Sync + 'static;

    /// Returns the policy ID
    /// 
    /// For versioned policies, use the `policy_id_with_version` helper method
    /// from your parent module to generate the appropriate ID.
    fn policy_id() -> &'static str;
    
    /// If this policy supports versioning, this method can be implemented to
    /// provide the version of the policy. Default implementation returns None.
    fn version() -> Option<&'static str> {
        None
    }
    
    /// Creates a new instance of the policy with the provided configuration
    async fn new(config: Self::Config) -> Result<Self::PolicyType, String>;
    
    /// Validates the policy configuration
    fn validate_config(config: &Self::Config) -> Result<(), String>;
}

#[async_trait]
pub trait Policy: Send + Sync + 'static {
    async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult;
}
