use axum::{
    routing::MethodRouter,
    Router,
};

pub struct PolicyRouteBuilder {
    base_path: String,
}

impl PolicyRouteBuilder {
    pub fn new(provider: &str, category: &str, policy_name: &str, version: &str) -> Self {
        let base_path = format!("/_admin/{}/{}/{}/{}", provider, category, policy_name, version);
        tracing::debug!("Created PolicyRouteBuilder with base path: {}", base_path);
        Self { base_path }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }
}

pub struct RouteRegistration {
    pub relative_path: String,
    pub handler: MethodRouter,
}

#[derive(Clone)]
pub struct PolicyRouter {
    routes: Vec<(String, MethodRouter)>,
}

impl PolicyRouter {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    pub fn register_routes(&mut self, registrations: Vec<RouteRegistration>, base_path: &str) {
        tracing::debug!("Registering routes for base path: {}", base_path);
        for registration in registrations {
            // Ensure the relative path is properly formatted
            let relative_path = if registration.relative_path.starts_with('/') {
                registration.relative_path
            } else {
                format!("/{}", registration.relative_path)
            };

            // Construct the full path here, where we have control
            let full_path = format!("{}{}", base_path, relative_path);
            
            // Register both versions of the path
            let path_without_slash = full_path.trim_end_matches('/').to_string();
            let path_with_slash = format!("{}/", path_without_slash);
            
            // Store both route handlers
            self.routes.push((path_without_slash.clone(), registration.handler.clone()));
            self.routes.push((path_with_slash, registration.handler));
            
            // Log the registered routes
            tracing::info!("Registered policy routes: {} and {}/", path_without_slash, path_without_slash);
        }
    }

    pub fn into_router(self) -> Router {
        let mut router = Router::new();
        let route_count = self.routes.len();
        
        for (path, handler) in self.routes {
            tracing::debug!("Adding route to router: {}", path);
            router = router.route(&path, handler);
        }
        
        tracing::debug!("Policy router built with {} routes", route_count);
        router
    }
}