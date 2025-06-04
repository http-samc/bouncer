# Authentication Policies

Authentication policies in Bouncer are responsible for verifying the identity of users and setting their roles. This document outlines the requirements and best practices for implementing authentication policies.

## Header Management

### Clearing Bouncer Headers

Before any request enters the policy chain, Bouncer automatically clears all headers that start with `x-bouncer-`. This is a security measure to prevent header injection attacks and ensure that roles are always set by trusted authentication policies.

### Setting Roles

All authentication policies MUST set the `x-bouncer-role` header on the incoming request. This header is used by authorization policies (like RBAC) to determine what actions the authenticated user is allowed to perform.

Example of setting the role header in a policy:

```rust
// Add role to request headers
let mut request = request;
request.headers_mut().insert(
    header::HeaderName::from_static("x-bouncer-role"),
    header::HeaderValue::from_str(&role).unwrap_or_else(|_| {
        tracing::error!("Failed to create header value for role: {}", role);
        header::HeaderValue::from_static("unknown")
    }),
);
```

## Policy Chain Order

Authentication policies should typically be placed before authorization policies in the policy chain. This ensures that:

1. The user's identity is verified first
2. The role is set before any authorization checks
3. Authorization policies can rely on the presence of the `x-bouncer-role` header

Example configuration:

```json
{
  "policies": [
    {
      "provider": "@bouncer/auth/authentication/bearer/v1",
      "parameters": {
        "db_provider": "mysql",
        "token_validation_query": "SELECT role FROM users WHERE token = ?"
      }
    },
    {
      "provider": "@bouncer/auth/authorization/rbac/v1",
      "parameters": {
        "route_roles": {
          "/api/users/*": ["admin", "user_manager"],
          "/api/public/**": ["admin", "user", "guest"]
        }
      }
    }
  ]
}
```

## Best Practices

1. **Role Validation**: Validate roles against a known set of valid roles before setting them in the header.

2. **Error Handling**: If authentication fails, return a 401 Unauthorized response with an appropriate error message.

3. **Logging**: Log authentication failures for security monitoring, but be careful not to log sensitive information.

4. **Database Integration**: When using database authentication, ensure proper error handling and connection management.

5. **Token Security**: If using tokens, ensure they are properly validated and not expired.

## Example Implementation

Here's a simplified example of an authentication policy that sets the role:

```rust
async fn process(&self, request: Request<Body>) -> PolicyResult {
    // Authenticate the user...
    let role = match authenticate_user(request).await {
        Ok(role) => role,
        Err(_) => {
            return PolicyResult::Terminate(
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Authentication failed"))
                    .unwrap(),
            );
        }
    };

    // Set the role header
    let mut request = request;
    request.headers_mut().insert(
        header::HeaderName::from_static("x-bouncer-role"),
        header::HeaderValue::from_str(&role).unwrap_or_else(|_| {
            tracing::error!("Failed to create header value for role: {}", role);
            header::HeaderValue::from_static("unknown")
        }),
    );

    PolicyResult::Continue(request)
}
```
