use crate::get_custom_policies;
use crate::policy::providers::bouncer::auth::bearer::BearerAuthPolicyFactory;
use crate::policy::registry::PolicyRegistry;
use crate::policy::PolicyChainExt;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use axum_server::Server;
use reqwest;
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

pub async fn start_server(config: crate::config::Config) {
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
        // Match root path explicitly
        .route("/", {
            let client_clone = client.clone();
            let config_clone = Arc::clone(&config_for_handler);
            axum::routing::any(move |req| {
                let client = client_clone.clone();
                let config = config_clone.clone();
                async move { handler(req, client, config).await }
            })
        })
        // Match all other paths with wildcard - this correctly captures nested paths
        .route("/*path", {
            let client_clone = client.clone();
            let config_clone = Arc::clone(&config_for_handler);
            axum::routing::any(move |req| {
                let client = client_clone.clone();
                let config = config_clone.clone();
                async move { handler(req, client, config).await }
            })
        })
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
) -> Response<Body> {
    // Check if destination is configured
    if let Some(destination) = &config.server.destination_address {
        // Extract URI components we need to preserve
        let method = req.method().clone();
        let uri = req.uri();
        let path = uri.path();
        let query = uri.query().unwrap_or("");

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

        // Extract headers and body from the request
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in req.headers() {
            if let Ok(header_name) = reqwest::header::HeaderName::try_from(name.as_str()) {
                if let Ok(header_value) = reqwest::header::HeaderValue::try_from(value.as_bytes()) {
                    headers.insert(header_name, header_value);
                }
            }
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

// Function to register built-in policies
fn register_builtin_policies(registry: &mut PolicyRegistry) {
    registry.register_policy::<BearerAuthPolicyFactory>();
}

// Function to register user-provided custom policies
#[allow(clippy::needless_borrow)]
fn register_custom_policies(registry: &mut PolicyRegistry) {
    // Use a fully qualified path rather than an import
    for register_fn in get_custom_policies() {
        register_fn(registry);
    }
}
