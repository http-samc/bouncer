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
    pub token_prefix: Option<String>,
    pub token_validation_query: Option<String>,
    pub collection: Option<String>,
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

// SQL Implementation of the TokenDatabaseAdapter
#[cfg(feature = "sql")]
pub struct SqlTokenAdapter {
    client: Arc<sqlx::Pool<sqlx::Postgres>>,
    token_validation_query: String,
}

#[cfg(feature = "sql")]
impl SqlTokenAdapter {
    pub fn new(client: Arc<sqlx::Pool<sqlx::Postgres>>, token_validation_query: String) -> Self {
        Self {
            client,
            token_validation_query,
        }
    }
}

#[cfg(feature = "sql")]
#[async_trait]
impl TokenDatabaseAdapter for SqlTokenAdapter {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError> {
        let result = sqlx::query_scalar::<_, String>(&self.token_validation_query)
            .bind(token)
            .fetch_optional(&*self.client)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(result)
    }
}

// Redis Implementation of the TokenDatabaseAdapter
#[cfg(feature = "redis")]
pub struct RedisTokenAdapter {
    client: Arc<redis::Client>,
    token_prefix: String,
}

#[cfg(feature = "redis")]
impl RedisTokenAdapter {
    pub fn new(client: Arc<redis::Client>, token_prefix: String) -> Self {
        Self {
            client,
            token_prefix,
        }
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl TokenDatabaseAdapter for RedisTokenAdapter {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let key = format!("{}:{}", self.token_prefix, token);
        let role: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(role)
    }
}

// MongoDB Implementation of the TokenDatabaseAdapter
#[cfg(feature = "mongo")]
pub struct MongoTokenAdapter {
    client: Arc<mongodb::Client>,
    database: String,
    collection: String,
}

#[cfg(feature = "mongo")]
impl MongoTokenAdapter {
    pub fn new(client: Arc<mongodb::Client>, database: String, collection: String) -> Self {
        Self {
            client,
            database,
            collection,
        }
    }
}

#[cfg(feature = "mongo")]
#[async_trait]
impl TokenDatabaseAdapter for MongoTokenAdapter {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError> {
        let database = self.client.database(&self.database);
        let collection = database.collection::<mongodb::bson::Document>(&self.collection);

        let filter = mongodb::bson::doc! { "token": token };
        let result = collection.find_one(filter).await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        match result {
            Some(doc) => {
                match doc.get("role") {
                    Some(role) => {
                        let role_str = role.as_str()
                            .ok_or_else(|| DatabaseError::ConversionError("Role is not a string".to_string()))?;
                        Ok(Some(role_str.to_string()))
                    },
                    None => Ok(None),
                }
            },
            None => Ok(None),
        }
    }
}

// Policy factory for creating bearer auth policies
pub struct BearerAuthPolicyFactory;

#[async_trait]
impl PolicyFactory for BearerAuthPolicyFactory {
    type PolicyType = BearerAuthPolicy;
    type Config = BearerAuthConfig;

    fn policy_id() -> &'static str {
        "@bouncer/auth/bearer"
    }

    async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // If using database authentication, initialize the adapter
        let db_adapter = if let Some(db_provider) = &config.db_provider {
            // Get the global database configuration
            let db_config = match crate::GLOBAL_CONFIG.get() {
                Some(global_config) => &global_config.databases,
                None => return Err("Global configuration not initialized".to_string()),
            };

            // Initialize the appropriate adapter based on the db_provider
            match db_provider.as_str() {
                #[cfg(feature = "sql")]
                "sql" => {
                    if config.token_validation_query.is_none() {
                        return Err("token_validation_query is required when using SQL database".to_string());
                    }

                    // Validate SQL config exists
                    crate::database::validate_database_config(db_config, "sql")
                        .map_err(|e| e.to_string())?;

                    // Get SQL client
                    let sql_config = db_config.sql.as_ref()
                        .ok_or_else(|| "SQL configuration is required".to_string())?;

                    // Get SQL client asynchronously
                    let client = crate::database::get_sql_client(sql_config)
                        .await
                        .map_err(|e| e.to_string())?;

                    let token_validation_query = config.token_validation_query
                        .clone()
                        .ok_or_else(|| "token_validation_query is required".to_string())?;

                    let adapter = SqlTokenAdapter::new(client, token_validation_query);
                    Some(Arc::new(adapter) as Arc<dyn TokenDatabaseAdapter>)
                },

                #[cfg(feature = "redis")]
                "redis" => {
                    if config.token_prefix.is_none() {
                        return Err("token_prefix is required when using Redis database".to_string());
                    }

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

                    let token_prefix = config.token_prefix
                        .clone()
                        .ok_or_else(|| "token_prefix is required".to_string())?;

                    let adapter = RedisTokenAdapter::new(client, token_prefix);
                    Some(Arc::new(adapter) as Arc<dyn TokenDatabaseAdapter>)
                },

                #[cfg(feature = "mongo")]
                "mongo" => {
                    if config.collection.is_none() {
                        return Err("collection is required when using MongoDB database".to_string());
                    }

                    // Validate MongoDB config exists
                    crate::database::validate_database_config(db_config, "mongo")
                        .map_err(|e| e.to_string())?;

                    // Get MongoDB client
                    let mongo_config = db_config.mongo.as_ref()
                        .ok_or_else(|| "MongoDB configuration is required".to_string())?;

                    // Get MongoDB client asynchronously
                    let client = crate::database::get_mongo_client(mongo_config)
                        .await
                        .map_err(|e| e.to_string())?;

                    let collection = config.collection
                        .clone()
                        .ok_or_else(|| "collection is required".to_string())?;

                    let adapter = MongoTokenAdapter::new(
                        client,
                        mongo_config.database.clone(),
                        collection
                    );
                    Some(Arc::new(adapter) as Arc<dyn TokenDatabaseAdapter>)
                },

                #[allow(unreachable_patterns)]
                _ => return Err(format!("Unsupported or disabled database provider: {}", db_provider)),
            }
        } else {
            None
        };

        // If using static token authentication, validate that token is provided
        if db_adapter.is_none() && config.token.is_none() {
            return Err("Either token or db_provider must be specified".to_string());
        }

        Ok(BearerAuthPolicy { config, db_adapter })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        // Either a static token or a database provider is required
        if config.token.is_none() && config.db_provider.is_none() {
            return Err("Either token or db_provider must be specified".to_string());
        }

        // If using database authentication, validate required parameters
        if let Some(db_provider) = &config.db_provider {
            match db_provider.as_str() {
                "sql" => {
                    if config.token_validation_query.is_none() {
                        return Err("token_validation_query is required when using SQL database".to_string());
                    }

                    #[cfg(not(feature = "sql"))]
                    return Err("SQL support is not enabled. Rebuild with the 'sql' feature.".to_string());
                },
                "redis" => {
                    if config.token_prefix.is_none() {
                        return Err("token_prefix is required when using Redis database".to_string());
                    }

                    #[cfg(not(feature = "redis"))]
                    return Err("Redis support is not enabled. Rebuild with the 'redis' feature.".to_string());
                },
                "mongo" => {
                    if config.collection.is_none() {
                        return Err("collection is required when using MongoDB database".to_string());
                    }

                    #[cfg(not(feature = "mongo"))]
                    return Err("MongoDB support is not enabled. Rebuild with the 'mongo' feature.".to_string());
                },
                _ => return Err(format!("Unsupported database provider: {}", db_provider)),
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Policy for BearerAuthPolicy {
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
                Ok(Some(_role)) => {
                    // TODO: Add role to request extensions
                    true
                },
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
                    .body(Body::from("Unauthorized: Invalid Bearer token"))
                    .unwrap(),
            )
        }
    }
}
