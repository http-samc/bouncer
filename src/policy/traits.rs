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

    fn policy_id() -> &'static str;
    async fn new(config: Self::Config) -> Result<Self::PolicyType, String>;
    fn validate_config(config: &Self::Config) -> Result<(), String>;
}

#[async_trait]
pub trait Policy: Send + Sync + 'static {
    async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult;
}
