# Policy Development Guidelines

## Policy Structure

Policies in Bouncer follow a versioned directory structure:

```
src/policy/providers/<provider>/<category>/<name>/<version>.rs
```

For example, a bearer authentication policy would be:

```
src/policy/providers/bouncer/auth/bearer/v1.rs
```

## Creating a New Policy

1. **Choose a logical namespace**:

   - Choose a provider namespace (e.g., `bouncer` for core policies)
   - Select an appropriate category (e.g., `auth`, `rate_limit`, etc.)
   - Choose a descriptive name for your policy

2. **Create the directory structure**:

   ```bash
   mkdir -p src/policy/providers/bouncer/rate_limit/fixed/
   ```

3. **Create a mod.rs file**:

   ```rust
   pub mod v1;

   // Helper function for policy ID generation
   pub fn policy_id_with_version(version: &str) -> &'static str {
       match version {
           "v1" => "@bouncer/rate_limit/fixed/v1",
           _ => panic!("Unsupported version: {}", version)
       }
   }
   ```

4. **Implement your policy (v1.rs)**:

   ```rust
   use crate::policy::traits::{Policy, PolicyFactory, PolicyResult};
   use async_trait::async_trait;
   use axum::{body::Body, http::{Request, Response, StatusCode}};
   use serde::Deserialize;

   #[derive(Debug, Clone, Deserialize)]
   pub struct FixedRateLimitConfig {
       pub requests_per_minute: u32,
       pub response_message: Option<String>,
   }

   pub struct FixedRateLimitPolicy {
       config: FixedRateLimitConfig,
       // Other fields for rate limiting implementation
   }

   pub struct FixedRateLimitPolicyFactory;

   #[async_trait]
   impl PolicyFactory for FixedRateLimitPolicyFactory {
       type PolicyType = FixedRateLimitPolicy;
       type Config = FixedRateLimitConfig;

       fn policy_id() -> &'static str {
           crate::policy::providers::bouncer::rate_limit::fixed::policy_id_with_version("v1")
       }

       fn version() -> Option<&'static str> {
           Some("v1")
       }

       async fn new(config: Self::Config) -> Result<Self::PolicyType, String> {
           // Create policy instance
           Ok(FixedRateLimitPolicy { config })
       }

       fn validate_config(config: &Self::Config) -> Result<(), String> {
           if config.requests_per_minute == 0 {
               return Err("requests_per_minute must be greater than 0".to_string());
           }
           Ok(())
       }
   }

   #[async_trait]
   impl Policy for FixedRateLimitPolicy {
       async fn process(&self, request: Request<Body>) -> PolicyResult {
           // Implementation of rate limiting logic
           // This is just a placeholder example

           let allowed = true; // This would be your actual rate limiting logic

           if allowed {
               PolicyResult::Continue(request)
           } else {
               let message = self.config.response_message.clone()
                   .unwrap_or_else(|| "Rate limit exceeded".to_string());

               PolicyResult::Terminate(
                   Response::builder()
                       .status(StatusCode::TOO_MANY_REQUESTS)
                       .body(Body::from(message))
                       .unwrap()
               )
           }
       }
   }
   ```

5. **Register your policy** in `src/server.rs`:

   ```rust
   fn register_builtin_policies(registry: &mut PolicyRegistry) {
       // Existing policies
       registry.register_policy::<crate::policy::providers::bouncer::auth::bearer::v1::BearerAuthPolicyFactory>();

       // Register your new policy version
       registry.register_policy::<crate::policy::providers::bouncer::rate_limit::fixed::v1::FixedRateLimitPolicyFactory>();
   }
   ```

## Versioning Guidelines

### When to Create a New Version

Create a new version (e.g., v2) when making changes that are incompatible with the existing version, such as:

- Changing required configuration parameters
- Modifying the fundamental behavior of the policy
- Changing the structure of data added to or modified in the request
- Changing the response format or status codes
- Making security changes that might impact existing configurations

### Creating a New Version

1. **Create a new version file**:

   ```bash
   touch src/policy/providers/bouncer/rate_limit/fixed/v2.rs
   ```

2. **Implement the new version** with your changes

3. **Update the mod.rs file**:

   ```rust
   pub mod v1;
   pub mod v2;  // Add the new version

   // Update policy_id_with_version
   pub fn policy_id_with_version(version: &str) -> &'static str {
       match version {
           "v1" => "@bouncer/rate_limit/fixed/v1",
           "v2" => "@bouncer/rate_limit/fixed/v2",
           _ => panic!("Unsupported version: {}", version)
       }
   }
   ```

4. **Register the new version**:

   ```rust
   fn register_builtin_policies(registry: &mut PolicyRegistry) {
       // Register existing policies
       registry.register_policy::<crate::policy::providers::bouncer::auth::bearer::v1::BearerAuthPolicyFactory>();

       // Register both versions of your policy
       registry.register_policy::<crate::policy::providers::bouncer::rate_limit::fixed::v1::FixedRateLimitPolicyFactory>();
       registry.register_policy::<crate::policy::providers::bouncer::rate_limit::fixed::v2::FixedRateLimitPolicyFactory>();
   }
   ```

## Example Configuration

Users need to specify the policy version in their configuration:

```yaml
"@bouncer/rate_limit/fixed/v1":
  requests_per_minute: 100
  response_message: "Rate limit exceeded. Please try again later."
```

## Best Practices

1. **Version Incrementing**: Use sequential version numbers (v1, v2, v3, etc.)
2. **Documentation**: Document changes between versions clearly in the code comments
3. **Thorough Testing**: Create tests for each version to ensure they function correctly
4. **Migration Guides**: Provide migration guides in documentation when creating new versions
