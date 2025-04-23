# bouncer

[![Crates.io](https://img.shields.io/crates/v/bouncer.svg)](https://crates.io/crates/bouncer)
[![Docs.rs](https://docs.rs/bouncer/badge.svg)](https://docs.rs/bouncer)
[![CI](https://github.com/http-samc/bouncer/workflows/CI/badge.svg)](https://github.com/http-samc/bouncer/actions)

A configurable API gateway and proxy server with dynamic policy middleware.

## Overview

Bouncer is a lightweight API gateway that sits between clients and your backend services. It provides:

- **Policy-based middleware** for authentication, authorization, and more
- **Database integration** with PostgreSQL, Redis, and MongoDB
- **Extensible architecture** with support for custom policies
- **Request proxying** to backend services with token verification

For a comprehensive explanation of Bouncer's features and architecture, see [docs/ABOUT.md](docs/ABOUT.md).

## Quick Start

### Installation

```bash
# Install from crates.io
cargo install bouncer

# Or build from source
git clone https://github.com/http-samc/bouncer.git
cd bouncer
cargo build --release
```

### Running Bouncer

1. Create a configuration file `config.yaml`:

```yaml
server:
  port: 8080
  bind_address: "0.0.0.0"
  destination_address: "http://my-backend-api.com"

policies:
  - type: "@bouncer/auth/bearer"
    config:
      token: "my-secure-token"
```

2. Set the `BOUNCER_TOKEN` environment variable (optional but recommended):

```bash
export BOUNCER_TOKEN="your-secure-token-here"
```

3. Run Bouncer with your configuration:

```bash
bouncer --config config.yaml
```

## Documentation

See [ABOUT.md](docs/ABOUT.md) for a comprehensive explanation of Bouncer and additional resources.

## License

Licensed under the MIT license ([LICENSE-MIT](LICENSE-MIT)).

## Contribution

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
