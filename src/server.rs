use crate::policy::registry::PolicyRegistry;
use crate::policy::PolicyChainExt;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use axum_server::Server;
use reqwest;
use std::convert::TryFrom;
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use crate::GLOBAL_CONFIG;

pub async fn start_server(config: crate::config::Config) {
    // Store config in global cell for access from policies
    if GLOBAL_CONFIG.set(config.clone()).is_err() {
        tracing::warn!("Global config already set, using existing config");
    }

    // Check for BOUNCER_TOKEN environment variable
    let bouncer_token = match env::var("BOUNCER_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            tracing::warn!("BOUNCER_TOKEN environment variable not set. This may make your target API vulnerable to impersonation.");
            tracing::warn!("Using insecure default token 'secret'. Please set BOUNCER_TOKEN in production.");
            "secret".to_string()
        }
    };

    // Create policy registry and register all available policies
    let mut registry = PolicyRegistry::new();

    // Register built-in policies
    register_builtin_policies(&mut registry);

    // Register user-provided custom policies
    register_custom_policies(&mut registry);

    // Load external policies from plugins directory if it exists
    // This is kept for backward compatibility
    let plugins_dir = Path::new("plugins");
    if plugins_dir.exists() && plugins_dir.is_dir() {
        match registry.load_policies_from_directory(plugins_dir) {
            Ok(_) => tracing::info!("Loaded external policies from plugins directory"),
            Err(e) => tracing::warn!("Failed to load external policies: {}", e),
        }
    }

    // Build policy chain based on config file
    let policy_chain = registry
        .build_policy_chain(&config.policies)
        .await
        .expect("Failed to build policy chain");

    // Create a shared HTTP client for forwarding requests
    let client = reqwest::Client::builder()
        .build()
        .expect("Failed to create HTTP client");

    // Share config with handler
    let config = Arc::new(config);
    let config_for_handler = Arc::clone(&config);

    // Create Axum router with middleware for policies
    let app = Router::new()
        .route(
            "/{*path}",
            axum::routing::any(move |req| {
                // Clone the token for use in the handler
                let token = bouncer_token.clone();
                handler(req, client.clone(), config_for_handler.clone(), token)
            }),
        )
        .layer(policy_chain.into_layer());

    // Start the HTTP server
    let addr: SocketAddr = config
        .full_bind_address()
        .parse()
        .expect("Invalid bind address");

    tracing::info!("Starting server on {}", addr);

    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .expect("Server failed");
}

// Handler for processing requests after middleware executes
async fn handler(
    req: Request<Body>,
    client: reqwest::Client,
    config: Arc<crate::config::Config>,
    bouncer_token: String,
) -> Response<Body> {
    // Check if destination is configured
    if let Some(destination) = &config.server.destination_address {
        // Extract URI components we need to preserve
        let method = req.method().clone();
        let uri = req.uri();
        let path = uri.path();
        let query = uri.query().unwrap_or("");

        tracing::info!("Original request path: {}", path);

        // Construct the destination URL
        let url = {
            let destination_trimmed = destination.trim_end_matches('/');
            let path_trimmed = path.trim_start_matches('/');

            if path_trimmed.is_empty() {
                // Just the destination for root path
                destination_trimmed.to_string()
            } else if query.is_empty() {
                // No query parameters
                format!("{}/{}", destination_trimmed, path_trimmed)
            } else {
                // With query parameters
                format!("{}/{}?{}", destination_trimmed, path_trimmed, query)
            }
        };

        tracing::info!("Forwarding to URL: {}", url);

        // Extract headers and body from the request, filtering out bouncer-* headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in req.headers() {
            // Skip any header starting with 'bouncer' (case/whitespace insensitive)
            let header_str = name.as_str().to_lowercase();
            if header_str.starts_with("bouncer") {
                continue;
            }

            if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                if let Ok(header_value) = reqwest::header::HeaderValue::try_from(value.as_bytes()) {
                    headers.insert(header_name, header_value);
                }
            }
        }

        // Add bouncer-token header with our token
        if let Ok(token_value) = reqwest::header::HeaderValue::try_from(bouncer_token.as_bytes()) {
            headers.insert("bouncer-token", token_value);
        }

        // Convert the request body using axum's collect method
        let (_parts, body) = req.into_parts();
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes.to_vec(),
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to read request body"))
                    .unwrap();
            }
        };

        // Forward the request to the destination
        let proxy_request = match method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url).body(bytes),
            "PUT" => client.put(&url).body(bytes),
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url).body(bytes),
            "HEAD" => client.head(&url),
            "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
            _ => {
                return Response::builder()
                    .status(StatusCode::NOT_IMPLEMENTED)
                    .body(Body::from(format!("HTTP method {} not supported", method)))
                    .unwrap();
            }
        };

        // Set headers and send the request
        let response = match proxy_request.headers(headers).send().await {
            Ok(res) => res,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from(format!("Failed to forward request: {}", e)))
                    .unwrap();
            }
        };

        // Convert the response back to an Axum response
        // Convert reqwest::StatusCode to axum::http::StatusCode using its numeric value
        let status_code = StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response_builder = Response::builder().status(status_code);

        // Copy headers from the forwarded response
        for (name, value) in response.headers() {
            response_builder = response_builder.header(name.as_str(), value.as_bytes());
        }

        // Convert the response body
        let body = match response.bytes().await {
            Ok(bytes) => Body::from(bytes.to_vec()),
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to read response body"))
                    .unwrap();
            }
        };

        return response_builder.body(body).unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to construct response"))
                .unwrap()
        });
    }

    // If no destination is configured, return a default response
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("Hello from Bouncer!"))
        .unwrap()
}

// Register built-in policies
fn register_builtin_policies(registry: &mut PolicyRegistry) {
    // Only register the versioned implementations
    registry.register_policy::<crate::policy::providers::bouncer::auth::bearer::v1::BearerAuthPolicyFactory>();
    
    // Add other built-in policies here
}

// Register custom policies from global registry
fn register_custom_policies(registry: &mut PolicyRegistry) {
    for register_fn in crate::get_custom_policies() {
        register_fn(registry);
    }
}
