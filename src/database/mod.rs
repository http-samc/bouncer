use crate::config::{DatabasesConfig, MongoConfig, RedisConfig, SqlConfig};
use std::sync::Arc;

pub mod errors;
pub use errors::DatabaseError;

// Helper functions for getting database clients

#[cfg(feature = "sql")]
/// Get a SQL database client from configuration
pub async fn get_sql_client(config: &SqlConfig) -> Result<Arc<sqlx::Pool<sqlx::Postgres>>, DatabaseError> {
    if config.connection_url.is_empty() {
        return Err(DatabaseError::ConfigurationError("SQL connection URL is required".to_string()));
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.connection_pool_size.unwrap_or(5))
        .connect(&config.connection_url)
        .await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    Ok(Arc::new(pool))
}

#[cfg(not(feature = "sql"))]
/// Get a SQL database client from configuration (feature not enabled)
pub async fn get_sql_client(_config: &SqlConfig) -> Result<Arc<()>, DatabaseError> {
    Err(DatabaseError::ConfigurationError("SQL support is not enabled. Rebuild with the 'sql' feature.".to_string()))
}

#[cfg(feature = "redis")]
/// Get a Redis client from configuration
pub async fn get_redis_client(config: &RedisConfig) -> Result<Arc<redis::Client>, DatabaseError> {
    if config.connection_url.is_empty() {
        return Err(DatabaseError::ConfigurationError("Redis connection URL is required".to_string()));
    }

    let client = redis::Client::open(&config.connection_url[..])
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    // Test the connection
    let mut conn = client.get_async_connection().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    redis::cmd("PING").query_async::<_, String>(&mut conn).await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    Ok(Arc::new(client))
}

#[cfg(not(feature = "redis"))]
/// Get a Redis client from configuration (feature not enabled)
pub async fn get_redis_client(_config: &RedisConfig) -> Result<Arc<()>, DatabaseError> {
    Err(DatabaseError::ConfigurationError("Redis support is not enabled. Rebuild with the 'redis' feature.".to_string()))
}

#[cfg(feature = "mongo")]
/// Get a MongoDB client from configuration
pub async fn get_mongo_client(config: &MongoConfig) -> Result<Arc<mongodb::Client>, DatabaseError> {
    if config.connection_uri.is_empty() {
        return Err(DatabaseError::ConfigurationError("MongoDB connection URI is required".to_string()));
    }

    let client_options = mongodb::options::ClientOptions::parse(&config.connection_uri)
        .await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    let client = mongodb::Client::with_options(client_options)
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    // Test the connection
    client.list_database_names().await
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

    Ok(Arc::new(client))
}

#[cfg(not(feature = "mongo"))]
/// Get a MongoDB client from configuration (feature not enabled)
pub async fn get_mongo_client(_config: &MongoConfig) -> Result<Arc<()>, DatabaseError> {
    Err(DatabaseError::ConfigurationError("MongoDB support is not enabled. Rebuild with the 'mongo' feature.".to_string()))
}

/// Validate that the databases section of config contains required database
pub fn validate_database_config(config: &DatabasesConfig, db_provider: &str) -> Result<(), DatabaseError> {
    match db_provider {
        "sql" => {
            if config.sql.is_none() {
                return Err(DatabaseError::ConfigurationError(
                    "SQL database configuration is required but not provided".to_string(),
                ));
            }

            #[cfg(not(feature = "sql"))]
            return Err(DatabaseError::ConfigurationError(
                "SQL support is not enabled. Rebuild with the 'sql' feature.".to_string()
            ));
        },
        "redis" => {
            if config.redis.is_none() {
                return Err(DatabaseError::ConfigurationError(
                    "Redis database configuration is required but not provided".to_string(),
                ));
            }

            #[cfg(not(feature = "redis"))]
            return Err(DatabaseError::ConfigurationError(
                "Redis support is not enabled. Rebuild with the 'redis' feature.".to_string()
            ));
        },
        "mongo" => {
            if config.mongo.is_none() {
                return Err(DatabaseError::ConfigurationError(
                    "MongoDB database configuration is required but not provided".to_string(),
                ));
            }

            #[cfg(not(feature = "mongo"))]
            return Err(DatabaseError::ConfigurationError(
                "MongoDB support is not enabled. Rebuild with the 'mongo' feature.".to_string()
            ));
        },
        _ => {
            return Err(DatabaseError::ConfigurationError(
                format!("Unknown database provider: {}", db_provider)
            ));
        }
    }

    Ok(())
}