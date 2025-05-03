use crate::database::DatabaseError;
use crate::policy::routes::RouteRegistration;
use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    routing::get,
    Router,
    extract::Path,
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// Re-export the config type from v1
pub use super::v1::BearerAuthConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct BearerAuthManagedConfig {
    pub realm: Option<String>,
    pub token_key_prefix: String,
    pub token_key_salt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    roles: Vec<String>,
    owner: String,
}

// Policy implementation with Redis support
pub struct BearerAuthManagedPolicy {
    config: BearerAuthManagedConfig,
    redis_client: redis::Client,
}

impl BearerAuthManagedPolicy {
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.update(self.config.token_key_salt.as_bytes());
        BASE64.encode(hasher.finalize())
    }

    fn get_redis_key(&self, token: &str) -> String {
        format!("{}:{}", self.config.token_key_prefix, self.hash_token(token))
    }

    async fn get_token_data(&self, token: &str) -> Result<Option<TokenData>, DatabaseError> {
        let mut conn = self.redis_client.get_async_connection().await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let key = self.get_redis_key(token);
        let data: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        match data {
            Some(json_str) => {
                let token_data: TokenData = serde_json::from_str(&json_str)
                    .map_err(|e| DatabaseError::ConversionError(e.to_string()))?;
                Ok(Some(token_data))
            },
            None => Ok(None),
        }
    }
}

// Policy factory for creating managed bearer auth policies
pub struct BearerAuthManagedPolicyFactory;

#[async_trait]
impl PolicyFactory for BearerAuthManagedPolicyFactory {
    type PolicyType = BearerAuthManagedPolicy;
    type Config = BearerAuthManagedConfig;

    fn policy_id() -> &'static str {
        crate::policy::providers::bouncer::auth::bearer::policy_id_with_version("v1_managed")
    }

    fn version() -> Option<&'static str> {
        Some("v1_managed")
    }

    async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // Get the global database configuration
        let db_config = match crate::GLOBAL_CONFIG.get() {
            Some(global_config) => &global_config.databases,
            None => return Err("Global configuration not initialized".to_string()),
        };

        // Validate Redis config exists
        crate::database::validate_database_config(db_config, "redis")
            .map_err(|e| e.to_string())?;

        // Get Redis client
        let redis_config = db_config.redis.as_ref()
            .ok_or_else(|| "Redis configuration is required".to_string())?;

        // Get Redis client asynchronously
        let client = crate::database::get_redis_client(redis_config)
            .await
            .map_err(|e| e.to_string())?;

        Ok(BearerAuthManagedPolicy {
            config,
            redis_client: Arc::try_unwrap(client).map_err(|_| "Failed to unwrap Redis client".to_string())?,
        })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        if config.token_key_prefix.is_empty() {
            return Err("token_key_prefix cannot be empty".to_string());
        }
        if config.token_key_salt.is_empty() {
            return Err("token_key_salt cannot be empty".to_string());
        }
        Ok(())
    }
}

#[async_trait]
impl Policy for BearerAuthManagedPolicy {
    fn provider(&self) -> &'static str {
        "bouncer"
    }

    fn category(&self) -> &'static str {
        "auth"
    }

    fn name(&self) -> &'static str {
        "bearer"
    }

    fn version(&self) -> &'static str {
        "v1_managed"
    }

    fn register_routes(&self) -> Vec<RouteRegistration> {
        tracing::debug!("Registering routes for bearer auth policy v1_managed");
        vec![
            RouteRegistration {
                relative_path: "".to_string(), // Base path
                handler: get(|| async {
                    tracing::debug!("Bearer auth policy v1_managed handler called");
                    "Hello from Bearer Auth Policy v1_managed!"
                }),
            }
        ]
    }

    async fn process(&self, request: Request<Body>) -> PolicyResult {
        // Extract the Authorization header
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

        // Extract the token from the header
        let token = match auth_str.strip_prefix("Bearer ") {
            Some(t) => t,
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
                        .body(Body::from("Unauthorized: Invalid Bearer token format"))
                        .unwrap(),
                );
            }
        };

        // Get token data from Redis
        match self.get_token_data(token).await {
            Ok(Some(token_data)) => {
                // Add roles and owner to request headers
                let mut request = request;
                let headers = request.headers_mut();

                // Add roles as comma-separated list
                headers.insert(
                    "X-Auth-Roles",
                    token_data.roles.join(",").parse().unwrap(),
                );

                // Add owner
                headers.insert(
                    "X-Auth-Owner",
                    token_data.owner.parse().unwrap(),
                );

                PolicyResult::Continue(request)
            },
            Ok(None) => {
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
            },
            Err(e) => {
                tracing::error!("Redis authentication error: {}", e);
                PolicyResult::Terminate(
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Internal server error"))
                        .unwrap(),
                )
            }
        }
    }
}

// Add a router for the key validation endpoint
pub fn router(policy: Arc<BearerAuthManagedPolicy>) -> Router {
    Router::new()
        .route("/keys/:key", get(validate_key))
        .with_state(policy)
}

async fn validate_key(
    Path(key): Path<String>,
    State(policy): State<Arc<BearerAuthManagedPolicy>>,
) -> Result<Json<TokenData>, (StatusCode, String)> {
    match policy.get_token_data(&key).await {
        Ok(Some(token_data)) => Ok(Json(token_data)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Key not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}