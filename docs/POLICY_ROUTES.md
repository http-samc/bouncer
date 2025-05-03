# Policy Routes

Bouncer allows policies to register and handle their own HTTP routes. This enables policies to expose management APIs, status endpoints, or any other HTTP functionality they need.

## How It Works

Policies can implement the `register_routes` method from the `Policy` trait to define their own HTTP routes. These routes are automatically namespaced under the policy's path:

```
/_admin/{provider}/{category}/{name}/{version}/{relative_path}
```

For example, if a policy with provider `bouncer`, category `auth`, name `bearer`, and version `v1` registers a route with relative path `status`, the full path would be:

```
/_admin/bouncer/auth/bearer/v1/status
```

## Implementing Policy Routes

To add routes to your policy, implement the `register_routes` method:

```rust
fn register_routes(&self) -> Vec<RouteRegistration> {
    vec![
        RouteRegistration {
            relative_path: "status".to_string(),
            handler: get(|| async {
                "Policy status endpoint"
            }),
        }
    ]
}
```

Each `RouteRegistration` contains:

- `relative_path`: The path relative to the policy's namespace
- `handler`: An Axum handler function that processes the request

## Example: Bearer Auth Policy

The Bearer Auth policy provides a good example of how to implement policy routes. It comes in two versions:

1. `v1`: The base version that only handles authentication
2. `v1-managed`: An extension that adds management routes while inheriting all authentication functionality

Here's how the managed version is implemented:

```rust
pub struct BearerAuthManagedPolicy {
    inner: BearerAuthPolicy,  // Inherits from the base policy
}

impl Policy for BearerAuthManagedPolicy {
    // ... other trait methods ...

    fn register_routes(&self) -> Vec<RouteRegistration> {
        vec![
            RouteRegistration {
                relative_path: "".to_string(),  // Base path
                handler: get(|| async {
                    "Hello from Bearer Auth Policy v1-managed!"
                }),
            }
        ]
    }

    // Delegate to the inner policy's process method
    async fn process(&self, request: Request<Body>) -> PolicyResult {
        self.inner.process(request).await
    }
}
```

## Using Policy Routes

To use a policy with routes, specify it in your Bouncer configuration:

```yaml
policies:
  - provider: "@bouncer/auth/bearer/v1-managed"
    parameters:
      # ... policy configuration ...
```

Then you can access the policy's routes at:

```
http://localhost:8000/_admin/bouncer/auth/bearer/v1-managed/
```

## Best Practices

1. **Namespace Your Routes**: Always use relative paths in your route registrations. Bouncer will automatically prefix them with the policy's namespace.

2. **Keep Routes Focused**: Only register routes that are directly related to the policy's functionality.

3. **Document Your Routes**: Clearly document what routes your policy exposes and how to use them.

4. **Version Your Routes**: If you make breaking changes to your routes, create a new version of your policy.

5. **Handle Errors Gracefully**: Make sure your route handlers properly handle errors and return appropriate HTTP status codes.

## Route Registration Order

Bouncer registers policy routes in the following order:

1. Policy routes are registered first
2. The catch-all route for forwarding is registered last

This ensures that policy routes take precedence over the forwarding route.

## Security Considerations

- All policy routes are automatically prefixed with `/_admin/`
- Policy routes are not subject to the policy chain (they bypass policy processing)
- Make sure to implement proper authentication and authorization in your route handlers if needed
