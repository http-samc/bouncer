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
