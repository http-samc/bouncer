# Simple Custom Policy Example

This example demonstrates the simplified approach to creating and using custom policies with Bouncer.

## Key Features

- No need for plugins directory or dynamic libraries
- Just add Bouncer as a dependency
- Register your policies directly in your code
- Start the server with a single line of code

## How It Works

1. Add `bouncer` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
bouncer = "0.1.0" # Use the latest version
```

2. Create your custom policy and factory implementation:

```rust
use bouncer::{Policy, PolicyFactory, PolicyResult};

// Define your policy...
pub struct MyCustomPolicy { /* ... */ }

impl Policy for MyCustomPolicy {
    // Policy implementation...
}

// Define your factory...
pub struct MyCustomPolicyFactory;

impl PolicyFactory for MyCustomPolicyFactory {
    // Factory implementation...
}
```

3. Register your policy and start the server:

```rust
#[tokio::main]
async fn main() {
    // Register your custom policy
    bouncer::register_custom_policy(|registry| {
        registry.register_policy::<MyCustomPolicyFactory>();
    });

    // Start the server with your config file
    bouncer::start_with_config("config.yaml").await;
}
```

## Running This Example

To run this example:

```sh
cargo run
```

Then make requests to `http://127.0.0.1:8080` to see the rate limiting policy in action.
