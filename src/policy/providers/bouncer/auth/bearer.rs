use crate::policy::{traits::*};
use axum::{http::{Request}, response::{Response}};
use async_trait::async_trait;

#[derive(serde::Deserialize)]
pub struct BearerAuthConfig {
    pub header_name: String,
}

pub struct BearerAuthPolicy {
    header_name: String,
}

#[async_trait]
impl Policy for BearerAuthPolicy {
    type Config = BearerAuthConfig;

    fn new(config: Self::Config) -> Result<Self, String> {
        Ok(Self { header_name: config.header_name })
    }

    async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult {
        if request.headers().contains_key(&self.header_name) {
            PolicyResult::Continue(request)
        } else {
            let response = Response::<axum::body::Body>::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
            PolicyResult::Terminate(response)
        }
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        if config.header_name.is_empty() {
            Err("header_name cannot be empty".to_string())
        } else {
            Ok(())
        }
    }
}
