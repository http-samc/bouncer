# Adding Database Support to Policies

This guide explains how to integrate database support into Bouncer policies, using the Bearer Authentication policy as an example.

## Overview

The process involves:

1. Defining a database adapter trait
2. Adding database configuration to your policy
3. Implementing database-specific adapters
4. Configuring the policy factory
5. Using the adapter in the policy implementation

## Step 1: Define a Domain-Specific Database Adapter Trait

First, define a trait that specifies what database operations your policy needs:

```rust
#[async_trait]
pub trait TokenDatabaseAdapter: Send + Sync + 'static {
    async fn get_role_from_token(&self, token: &str) -> Result<Option<String>, DatabaseError>;
}
```

This trait should:

- Be specific to your policy's domain (e.g., token validation)
- Use `async_trait` for async functions
- Return appropriate types for your use case
- Include the `Send + Sync + 'static` bounds for thread safety

## Step 2: Add Database Configuration to Policy Config

Modify your policy configuration to include database options:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct BearerAuthConfig {
    pub token: Option<String>,             // Static token (optional)
    pub realm: Option<String>,             // Auth realm
    pub db_provider: Option<String>,       // Database type: "sql", "redis", "mongo"
    pub token_prefix: Option<String>,      // For Redis
    pub token_validation_query: Option<String>, // For SQL
    pub collection: Option<String>,        // For MongoDB
}
```

Include any database-specific configuration fields needed by each adapter type.

## Step 3: Implement Database-Specific Adapters

For each supported database, create an adapter struct and implement the trait:

### SQL Example

```rust
// 1. Define the adapter struct
#[cfg(feature = "sql")]
pub struct SqlTokenAdapter {
    client: Arc<sqlx::Pool<sqlx::Postgres>>,
    token_validation_query: String,
}

// 2. Implement constructor
#[cfg(feature = "sql")]
impl SqlTokenAdapter {
    pub fn new(client: Arc<sqlx::Pool<sqlx::Postgres>>, token_validation_query: String) -> Self {
        Self {
            client,
            token_validation_query,
        }
    }
}

// 3. Implement the domain-specific adapter trait
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
```

### Redis Example

```rust
// 1. Define the adapter struct
#[cfg(feature = "redis")]
pub struct RedisTokenAdapter {
    client: Arc<redis::Client>,
    token_prefix: String,
}

// 2. Implement constructor
#[cfg(feature = "redis")]
impl RedisTokenAdapter {
    pub fn new(client: Arc<redis::Client>, token_prefix: String) -> Self {
        Self {
            client,
            token_prefix,
        }
    }
}

// 3. Implement the domain-specific adapter trait
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
```

## Step 4: Modify the Policy Struct

Add a field to store the database adapter:

```rust
pub struct BearerAuthPolicy {
    config: BearerAuthConfig,
    db_adapter: Option<Arc<dyn TokenDatabaseAdapter>>,
}
```

## Step 5: Configure the Policy Factory

In your `PolicyFactory` implementation, add code to initialize the appropriate adapter:

```rust
impl PolicyFactory for BearerAuthPolicyFactory {
    // ... existing code ...

    fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
        // Initialize the database adapter if db_provider is specified
        let db_adapter = if let Some(db_provider) = &config.db_provider {
            // Get the global database configuration
            let db_config = match crate::GLOBAL_CONFIG.get() {
                Some(global_config) => &global_config.databases,
                None => return Err("Global configuration not initialized".to_string()),
            };

            match db_provider.as_str() {
                #[cfg(feature = "sql")]
                "sql" => {
                    // 1. Validate required configuration
                    if config.token_validation_query.is_none() {
                        return Err("token_validation_query is required when using SQL database".to_string());
                    }

                    // 2. Validate database connection config exists
                    crate::database::validate_database_config(db_config, "sql")
                        .map_err(|e| e.to_string())?;

                    // 3. Get client from global connection pool
                    let sql_config = db_config.sql.as_ref()
                        .ok_or_else(|| "SQL configuration is required".to_string())?;

                    let client = tokio::runtime::Handle::current().block_on(async {
                        crate::database::get_sql_client(sql_config)
                            .await
                            .map_err(|e| e.to_string())
                    })?;

                    // 4. Create adapter with required parameters
                    let token_validation_query = config.token_validation_query
                        .clone()
                        .ok_or_else(|| "token_validation_query is required".to_string())?;

                    let adapter = SqlTokenAdapter::new(client, token_validation_query);
                    Some(Arc::new(adapter) as Arc<dyn TokenDatabaseAdapter>)
                },

                // ... other database types ...

                _ => return Err(format!("Unsupported database provider: {}", db_provider)),
            }
        } else {
            None
        };

        // Create policy with the adapter
        Ok(BearerAuthPolicy { config, db_adapter })
    }

    fn validate_config(config: &Self::Config) -> Result<(), String> {
        // Validate basic requirements
        if config.token.is_none() && config.db_provider.is_none() {
            return Err("Either token or db_provider must be specified".to_string());
        }

        // Validate database-specific requirements
        if let Some(db_provider) = &config.db_provider {
            match db_provider.as_str() {
                "sql" => {
                    if config.token_validation_query.is_none() {
                        return Err("token_validation_query is required when using SQL database".to_string());
                    }

                    #[cfg(not(feature = "sql"))]
                    return Err("SQL support is not enabled. Rebuild with the 'sql' feature.".to_string());
                },
                // ... other database types ...
            }
        }

        Ok(())
    }
}
```

## Step 6: Use the Adapter in the Policy

In your policy's `process` method, use the adapter:

```rust
#[async_trait]
impl Policy for BearerAuthPolicy {
    async fn process(&self, request: Request<Body>) -> PolicyResult {
        // ... extract token from request ...

        // Authenticate using either static token or database
        let is_authenticated = if let Some(db_adapter) = &self.db_adapter {
            // Use the database adapter to validate token
            match db_adapter.get_role_from_token(token).await {
                Ok(Some(role)) => {
                    // Optional: Store role in request extensions
                    // request.extensions_mut().insert(UserRole(role));
                    true
                },
                Ok(None) => false,
                Err(e) => {
                    tracing::error!("Database authentication error: {}", e);
                    false
                }
            }
        } else if let Some(static_token) = &self.config.token {
            // Fallback to static token authentication
            token == static_token
        } else {
            false
        };

        if is_authenticated {
            PolicyResult::Continue(request)
        } else {
            // Return unauthorized response
            PolicyResult::Terminate(/* ... */)
        }
    }
}
```

## Configuration Example

Example configuration for a bearer policy with SQL database:

```json
{
  "type": "@bouncer/auth/bearer",
  "config": {
    "db_provider": "sql",
    "token_validation_query": "SELECT role FROM users WHERE api_token = $1",
    "realm": "api"
  }
}
```

Example configuration for a bearer policy with Redis database:

```json
{
  "type": "@bouncer/auth/bearer",
  "config": {
    "db_provider": "redis",
    "token_prefix": "api:token",
    "realm": "api"
  }
}
```

## Best Practices

1. **Error Handling**: Properly handle database errors and avoid exposing internal error details to clients
2. **Feature Flags**: Use Cargo features to make database support optional
3. **Configuration Validation**: Always validate all required configuration fields are present
4. **Security**: Use parameterized queries for SQL to prevent injection
5. **Testing**: Write tests for each database adapter
6. **Documentation**: Document expected database schema/structure for each adapter
