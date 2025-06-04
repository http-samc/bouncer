use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Map of route patterns to allowed roles
    /// Route patterns can use glob syntax (e.g., "/api/*", "/users/**")
    pub route_roles: HashMap<String, Vec<String>>,
}

pub struct RbacPolicy {
    config: Arc<RbacConfig>,
}

#[derive(Default)]
pub struct RbacPolicyFactory;

impl PolicyFactory for RbacPolicyFactory {
    type PolicyType = RbacPolicy;
    type Config = RbacConfig;

    fn policy_id() -> &'static str {
        crate::policy::providers::bouncer::authorization::rbac::policy_id_with_version("v1")
    }

    fn version() -> Option<&'static str> {
        Some("v1")
    }

    fn new<'a>(
        config: Self::Config,
    ) -> Pin<Box<dyn futures::Future<Output = Result<Self::PolicyType, String>> + Send + 'a>> {
        Box::pin(async move {
            // Validate that at least one route is configured
            if config.route_roles.is_empty() {
                return Err("At least one route must be configured".to_string());
            }

            // Validate all route patterns
            for pattern_str in config.route_roles.keys() {
                Pattern::new(pattern_str)
                    .map_err(|e| format!("Invalid route pattern '{}': {}", pattern_str, e))?;
            }

            Ok(RbacPolicy {
                config: Arc::new(config),
            })
        })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        // Validate that we have at least one route configuration
        if config.route_roles.is_empty() {
            return Err("At least one route role mapping is required".to_string());
        }

        // Validate all route patterns
        for pattern_str in config.route_roles.keys() {
            Pattern::new(pattern_str)
                .map_err(|e| format!("Invalid route pattern '{}': {}", pattern_str, e))?;
        }

        Ok(())
    }
}

#[async_trait]
impl Policy for RbacPolicy {
    fn provider(&self) -> &'static str {
        "bouncer"
    }

    fn category(&self) -> &'static str {
        "authorization"
    }

    fn name(&self) -> &'static str {
        "rbac"
    }

    fn version(&self) -> &'static str {
        "v1"
    }

    async fn process(&self, request: Request<Body>) -> PolicyResult {
        let path = request.uri().path();
        let role = match request.headers().get("x-bouncer-role") {
            Some(role) => match role.to_str() {
                Ok(role) => {
                    tracing::info!("RBAC Policy: Processing request for path '{}' with role '{}'", path, role);
                    role
                },
                Err(_) => {
                    tracing::error!("RBAC Policy: Invalid role header format");
                    return PolicyResult::Terminate(
                        Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Body::from("Invalid role header"))
                            .unwrap(),
                    );
                }
            },
            None => {
                tracing::error!("RBAC Policy: No role header found in request");
                return PolicyResult::Terminate(
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::from("No role header found"))
                        .unwrap(),
                );
            }
        };

        // Check if the role has access to the requested path
        let has_access = self.config.route_roles.iter().any(|(pattern_str, roles)| {
            let pattern = Pattern::new(pattern_str).unwrap_or_else(|_| {
                tracing::error!("Invalid glob pattern: {}", pattern_str);
                Pattern::new("*").unwrap() // Default to matching nothing
            });

            let matches = pattern.matches(path) && roles.contains(&role.to_string());
            if matches {
                tracing::info!("RBAC Policy: Role '{}' has access to path '{}' via pattern '{}'", role, path, pattern_str);
            }
            matches
        });

        if !has_access {
            tracing::warn!("RBAC Policy: Access denied for role '{}' to path '{}'", role, path);
            return PolicyResult::Terminate(
                Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap(),
            );
        }

        tracing::info!("RBAC Policy: Access granted for role '{}' to path '{}'", role, path);
        PolicyResult::Continue(request)
    }
}
