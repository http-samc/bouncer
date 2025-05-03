use async_trait::async_trait;
use axum::http::{Request, Response};
use axum::body::Body;
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
pub trait Policy: Send + Sync {
    /// Returns the provider ID of the policy
    fn provider(&self) -> &'static str;

    /// Returns the category of the policy
    fn category(&self) -> &'static str;

    /// Returns the name of the policy
    fn name(&self) -> &'static str;

    /// Returns the version of the policy
    fn version(&self) -> &'static str;

    /// Register routes for the policy. Returns a vector of route registrations.
    /// Each registration contains a relative path and a handler.
    /// The paths will be automatically prefixed with the policy's namespace.
    fn register_routes(&self) -> Vec<crate::policy::routes::RouteRegistration> {
        vec![]
    }

    /// Process the request. This method is optional - policies can choose to only register routes.
    /// If not implemented, the policy will not be added to the policy chain.
    async fn process(&self, request: Request<Body>) -> PolicyResult {
        PolicyResult::Continue(request)
    }

    /// Returns true if the policy processes requests (i.e., implements process)
    fn processes_requests(&self) -> bool {
        true
    }
}
