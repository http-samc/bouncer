# About Bouncer

Bouncer is a configurable API gateway and proxy server written in Rust that sits between clients and your backend services. It provides a flexible policy-based middleware architecture that allows you to apply authentication, authorization, rate limiting, and other policies to incoming requests before they reach your backend services.

## Core Functionality

### API Gateway and Proxy

Bouncer's primary function is to accept HTTP requests from clients, apply configured policies, and then forward valid requests to your backend services. This architecture allows you to:

1. **Centralize cross-cutting concerns** like authentication and authorization
2. **Shield your backend services** from direct exposure to the internet
3. **Transform and normalize requests** before they reach your services
4. **Apply consistent policies** across your entire API surface

### Request Flow

1. Client sends a request to Bouncer
2. Bouncer applies configured policy chain (authentication, rate limiting, etc.)
3. If any policy rejects the request, Bouncer returns an appropriate error response
4. If all policies approve the request, Bouncer forwards it to the configured destination
5. Bouncer adds a `bouncer-token` header for verification by the backend service
6. Bouncer receives the response from the backend and forwards it to the client

## Features

### Policy-Based Architecture

Bouncer's core design revolves around configurable policies. Each policy is a self-contained piece of middleware that can:

- Examine and modify requests
- Approve or reject requests based on specific criteria
- Access external resources (databases, caches, etc.) for decision-making
- Add context for downstream policies

Policies are chained together and executed in sequence for each request.

### Built-in Policies

Bouncer includes several built-in policies out of the box:

- **Bearer Authentication**: Validates JWT or API tokens against a database or static configuration
- **Role-Based Access Control**: Restricts access based on user roles
- **Rate Limiting**: Prevents abuse by limiting request frequency
- **IP Filtering**: Restricts access based on source IP addresses

### Database Integration

Bouncer supports integrating with various database types to authenticate requests:

- **SQL (PostgreSQL/MySQL)**: Use SQL queries to validate tokens and retrieve roles
- **Redis**: Fast key-value lookups for token validation
- **MongoDB**: Document-based token validation

Each policy that supports database integration defines its own interface requirements. For example, the Bearer Authentication policy can:

- Use SQL databases with a custom token validation query
- Use Redis with a configurable token prefix
- Use MongoDB with a specified collection

_See [the full documentation](USING_DATABASES.md) for details._

### Bouncer Token Authentication

To ensure that your backend services only accept requests that have passed through Bouncer, each forwarded request includes a `bouncer-token` header with a configurable secret value.

1. Set the `BOUNCER_TOKEN` environment variable with a secure token
2. Bouncer strips all `bouncer-*` headers from incoming requests
3. Bouncer adds the `bouncer-token` header to validated requests
4. Your backend service validates this token to ensure requests came from your trusted Bouncer instance

_See [the full documentation](BOUNCER_TOKEN.md) for details._

### Environment Variable Configuration

Bouncer supports reading configuration values from environment variables, providing flexibility for deployment in various environments:

```yaml
databases:
  mysql:
    connection_url: "ENV.MYSQL_URL"
    max_connections: 5

server:
  port: 8000
  destination_address: "ENV.API_DESTINATION"
```

In this example, Bouncer will replace `ENV.MYSQL_URL` and `ENV.API_DESTINATION` with the values of those environment variables.

### Extensibility

Bouncer can be extended with custom policies:

1. Add `bouncer` as a dependency
2. Implement the `Policy` and `PolicyFactory` traits
3. Register your policy with the policy registry
4. Configure your policy in your configuration file

## Configuration

Bouncer is configured using a YAML file that defines:

1. **Server settings**: Port, bind address, destination address
2. **Database connections**: Connection details for supported databases
3. **Policy chain**: The sequence of policies to apply to each request

Example configuration:

```yaml
server:
  port: 8080
  bind_address: "0.0.0.0"
  destination_address: "http://my-backend-api.com"

databases:
  sql:
    connection_url: "postgres://user:password@localhost:5432/mydb"
    connection_pool_size: 5

policies:
  - type: "@bouncer/auth/bearer"
    config:
      db_provider: "sql"
      token_validation_query: "SELECT role FROM tokens WHERE id = $1 LIMIT 1;"
```

## Security Considerations

When deploying Bouncer, consider the following best practices:

1. **Set a secure `BOUNCER_TOKEN`**: This prevents unauthorized services from bypassing your gateway
2. **Use HTTPS**: Configure TLS for both client-to-Bouncer and Bouncer-to-backend communication
3. **Validate inputs**: Configure policies to validate request parameters before forwarding
4. **Limit exposure**: Deploy Bouncer in a network that restricts direct access to your backend services
5. **Monitor logs**: Bouncer logs policy decisions which can help detect potential security issues

## Database Integration Details

For complete details on configuring database integration with policies, see [DATABASE_POLICY_INTEGRATION.md](DATABASE_POLICY_INTEGRATION.md).

## Token Authentication Details

For complete details on configuring and using the Bouncer token authentication mechanism, see [BOUNCER_TOKEN.md](BOUNCER_TOKEN.md).
