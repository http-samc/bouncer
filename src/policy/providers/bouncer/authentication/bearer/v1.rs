use crate::database::DatabaseError;
use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct BearerAuthConfig {
    pub token: Option<String>,
    pub realm: Option<String>,
    pub db_provider: Option<String>,
    pub token_validation_query: Option<String>,
}

// Define the database adapter trait specific to the bearer auth policy
#[async_trait]
pub trait TokenDatabaseAdapter: Send + Sync + 'static {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError>;
}

// Policy implementation with optional database support
pub struct BearerAuthPolicy {
    config: BearerAuthConfig,
    db_adapter: Option<Arc<dyn TokenDatabaseAdapter>>,
}

// MySQL Implementation of the TokenDatabaseAdapter
pub struct MySqlTokenAdapter {
    client: Arc<sqlx::Pool<sqlx::MySql>>,
    token_validation_query: String,
}

impl MySqlTokenAdapter {
    pub fn new(client: Arc<sqlx::Pool<sqlx::MySql>>, token_validation_query: String) -> Self {
        Self {
            client,
            token_validation_query,
        }
    }
}

#[async_trait]
impl TokenDatabaseAdapter for MySqlTokenAdapter {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError> {
        let result = sqlx::query_scalar::<_, String>(&self.token_validation_query)
            .bind(token)
            .fetch_optional(&*self.client)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(result)
    }
}

// Policy factory for creating bearer auth policies
pub struct BearerAuthPolicyFactory;

#[async_trait]
impl PolicyFactory for BearerAuthPolicyFactory {
    type PolicyType = BearerAuthPolicy;
    type Config = BearerAuthConfig;

    fn policy_id() -> &'static str {
        crate::policy::providers::bouncer::authentication::bearer::policy_id_with_version("v1")
    }

    fn version() -> Option<&'static str> {
        Some("v1")
    }

    async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // If using database authentication, initialize the adapter
        let db_adapter = if let Some(db_provider) = &config.db_provider {
            if db_provider != "mysql" {
                return Err("Only MySQL database provider is supported".to_string());
            }

            if config.token_validation_query.is_none() {
                return Err(
                    "token_validation_query is required when using MySQL database".to_string(),
                );
            }

            // Get the global database configuration
            let db_config = match crate::GLOBAL_CONFIG.get() {
                Some(global_config) => &global_config.databases,
                None => return Err("Global configuration not initialized".to_string()),
            };

            // Validate MySQL config exists
            crate::database::validate_database_config(db_config, "mysql")
                .map_err(|e| e.to_string())?;

            // Get MySQL client
            let mysql_config = db_config
                .mysql
                .as_ref()
                .ok_or_else(|| "MySQL configuration is required".to_string())?;

            // Get MySQL client asynchronously
            let client = crate::database::get_mysql_client(mysql_config)
                .await
                .map_err(|e| e.to_string())?;

            // Create the adapter
            Some(Arc::new(MySqlTokenAdapter::new(
                client,
                config.token_validation_query.clone().unwrap(),
            )) as Arc<dyn TokenDatabaseAdapter>)
        } else {
            None
        };

        Ok(BearerAuthPolicy {
            config,
            db_adapter,
        })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        // If using database authentication, validate required fields
        if let Some(db_provider) = &config.db_provider {
            if db_provider != "mysql" {
                return Err("Only MySQL database provider is supported".to_string());
            }

            if config.token_validation_query.is_none() {
                return Err(
                    "token_validation_query is required when using MySQL database".to_string(),
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Policy for BearerAuthPolicy {
    fn provider(&self) -> &'static str {
        "bouncer"
    }

    fn category(&self) -> &'static str {
        "authentication"
    }

    fn name(&self) -> &'static str {
        "bearer"
    }

    fn version(&self) -> &'static str {
        "v1"
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

        // Authenticate using either static token or database
        let is_authenticated = if let Some(db_adapter) = &self.db_adapter {
            // Authenticate using database
            match db_adapter.get_role_from_token(token).await {
                Ok(Some(role)) => {
                    // Add role to request headers
                    let mut request = request;
                    request.headers_mut().insert(
                        header::HeaderName::from_static("x-bouncer-role"),
                        header::HeaderValue::from_str(&role).unwrap_or_else(|_| {
                            tracing::error!("Failed to create header value for role: {}", role);
                            header::HeaderValue::from_static("unknown")
                        }),
                    );
                    return PolicyResult::Continue(request);
                }
                Ok(None) => false,
                Err(e) => {
                    tracing::error!("Database authentication error: {}", e);
                    false
                }
            }
        } else if let Some(static_token) = &self.config.token {
            // Authenticate using static token
            token == static_token
        } else {
            // No authentication method configured
            false
        };

        if is_authenticated {
            // Authentication successful, continue processing
            PolicyResult::Continue(request)
        } else {
            // Authentication failed
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
                    .body(Body::from("Unauthorized: Invalid token"))
                    .unwrap(),
            )
        }
    }
}
