# Bouncer Token Authentication

Bouncer includes a mechanism for destination APIs to verify that requests are coming from Bouncer. This feature adds a `bouncer-token` header to all forwarded requests.

## How it works

1. Bouncer first strips all headers starting with `bouncer` (case-insensitive and whitespace-insensitive)
2. After passing through the policy chain, Bouncer adds a new header called `bouncer-token` with a value from the `BOUNCER_TOKEN` environment variable
3. If the `BOUNCER_TOKEN` environment variable is not set, Bouncer will:
   - Print a warning explaining how the target API may be vulnerable
   - Use an insecure default token of `secret`

## Setup

### With BOUNCER_TOKEN set

```bash
# Set the token
export BOUNCER_TOKEN="your-secure-token-here"

# Run Bouncer
cargo run -- --config your-config.yaml
```

### Security Considerations

If you don't set the `BOUNCER_TOKEN` environment variable, Bouncer will use a default token of `secret`. This is insecure and should only be used for development purposes.

Your destination API should validate the `bouncer-token` header to ensure requests are coming from a trusted Bouncer instance.

## Example Validation in Destination API

### Node.js (Express)

```javascript
app.use((req, res, next) => {
  const bouncerToken = req.headers["bouncer-token"];

  if (!bouncerToken || bouncerToken !== process.env.EXPECTED_BOUNCER_TOKEN) {
    return res.status(403).json({ error: "Unauthorized request" });
  }

  next();
});
```

### Python (FastAPI)

```python
from fastapi import FastAPI, Header, HTTPException
import os

app = FastAPI()

@app.middleware("http")
async def verify_bouncer_token(request, call_next):
    bouncer_token = request.headers.get("bouncer-token")
    expected_token = os.environ.get("EXPECTED_BOUNCER_TOKEN")

    if not bouncer_token or bouncer_token != expected_token:
        raise HTTPException(status_code=403, detail="Unauthorized request")

    response = await call_next(request)
    return response
```

### Rust (Axum)

```rust
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use std::env;

async fn verify_bouncer_token(
    request: Request,
    next: Next
) -> Result<Response, StatusCode> {
    let bouncer_token = request
        .headers()
        .get("bouncer-token")
        .and_then(|token| token.to_str().ok());

    let expected_token = env::var("EXPECTED_BOUNCER_TOKEN").ok();

    match (bouncer_token, expected_token) {
        (Some(token), Some(expected)) if token == expected => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::FORBIDDEN),
    }
}

// Use in your router setup
let app = Router::new()
    // ... your routes
    .layer(middleware::from_fn(verify_bouncer_token));
```
