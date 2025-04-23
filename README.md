# bouncer

[![Crates.io](https://img.shields.io/crates/v/bouncer.svg)](https://crates.io/crates/bouncer)
[![Docs.rs](https://docs.rs/bouncer/badge.svg)](https://docs.rs/bouncer)
[![CI](https://github.com/http-samc/bouncer/workflows/CI/badge.svg)](https://github.com/http-samc/bouncer/actions)

A configurable API gateway and proxy server with dynamic policy middleware.

## Installation

### Cargo

- Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
- run `cargo install bouncer`

## Features

### Database Integration

Bouncer supports integrating with various database types to authenticate requests. The supported databases are:

- SQL (PostgreSQL)
- Redis
- MongoDB

To use database integration, you need to enable the appropriate feature flags in your `Cargo.toml`:

```toml
[dependencies]
bouncer = { version = "0.1.0", features = ["sql", "redis", "mongo"] }
```

You can also enable all database types with the `all-db` feature:

```toml
bouncer = { version = "0.1.0", features = ["all-db"] }
```

Then configure your database connections in your `config.yaml`:

```yaml
server:
  # server config

databases:
  redis:
    connection_url: "redis://localhost:6379"
    password: "optional_password"
    database: 0
  sql:
    connection_url: "postgres://user:password@localhost:5432/mydb"
    connection_pool_size: 5
  mongo:
    connection_uri: "mongodb://localhost:27017"
    database: "mydb"

# Example: Bearer authentication with SQL database
@bouncer/auth/bearer:
  db_provider: sql
  token_validation_query: SELECT role FROM tokens WHERE id = $1 LIMIT 1;

# Example: Bearer authentication with Redis
@bouncer/auth/bearer:
  db_provider: redis
  token_prefix: tokens

# Example: Bearer authentication with MongoDB
@bouncer/auth/bearer:
  db_provider: mongo
  collection: tokens
```

Each policy that supports database integration will define its own interface requirements.

### Custom Policies

Bouncer supports custom policies that can be integrated directly into your application. To create and use a custom policy:

1. Add `bouncer` as a dependency in your `Cargo.toml`
2. Implement the `Policy` and `PolicyFactory` traits for your custom policy
3. Register your policy with `register_custom_policy`
4. Start the server with `start_with_config`

Example:

```rust
use bouncer::{Policy, PolicyFactory, PolicyResult, register_custom_policy, start_with_config};

// Define your policy and factory...
pub struct MyCustomPolicy { /* ... */ }
pub struct MyCustomPolicyFactory;

// Implement the required traits...
impl Policy for MyCustomPolicy { /* ... */ }
impl PolicyFactory for MyCustomPolicyFactory { /* ... */ }

#[tokio::main]
async fn main() {
    // Register your custom policy
    register_custom_policy(|registry| {
        registry.register_policy::<MyCustomPolicyFactory>();
    });

    // Start the server with your config
    start_with_config("config.yaml").await;
}
```

See the `examples/simple-custom-policy` directory for a complete example.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
