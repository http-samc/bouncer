use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BearerAuthConfig {
    pub token: String,
    pub realm: Option<String>,
}

pub struct BearerAuthPolicy {
    config: BearerAuthConfig,
}

pub struct BearerAuthPolicyFactory;

impl PolicyFactory for BearerAuthPolicyFactory {
    type PolicyType = BearerAuthPolicy;
    type Config = BearerAuthConfig;

    fn policy_id() -> &'static str {
        "@bouncer/auth/bearer"
    }

    fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        Ok(BearerAuthPolicy { config })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        if config.token.is_empty() {
            Err("token cannot be empty".to_string())
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Policy for BearerAuthPolicy {
    async fn process(&self, request: Request<Body>) -> PolicyResult {
        // Check if the request has a valid Authorization header
        let auth_header = match request.headers().get(header::AUTHORIZATION) {
            Some(header) => header,
            None => {
                return PolicyResult::Terminate(
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(
                            header::WWW_AUTHENTICATE,
                            format!(
                                "Bearer realm=\"{}\"",
                                self.config.realm.as_deref().unwrap_or("api")
                            ),
                        )
                        .body(Body::from("Unauthorized: Bearer token required"))
                        .unwrap(),
                );
            }
        };

        // Parse the header value
        let auth_str = match auth_header.to_str() {
            Ok(s) => s,
            Err(_) => {
                return PolicyResult::Terminate(
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::from("Invalid Authorization header format"))
                        .unwrap(),
                );
            }
        };

        // Check if it's a Bearer token and if it matches our configured token
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            if token == self.config.token {
                // Token is valid, continue processing
                return PolicyResult::Continue(request);
            }
        }

        // Invalid token
        PolicyResult::Terminate(
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    header::WWW_AUTHENTICATE,
                    format!(
                        "Bearer realm=\"{}\"",
                        self.config.realm.as_deref().unwrap_or("api")
                    ),
                )
                .body(Body::from("Unauthorized: Invalid Bearer token"))
                .unwrap(),
        )
    }
}
