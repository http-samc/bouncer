use crate::policy::traits::{Policy, PolicyResult};
use axum::{
    body::Body,
    http::{Request, Response},
};
use futures::future::BoxFuture;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

// Our middleware layer
#[derive(Clone)]
pub struct PolicyLayer {
    policies: Arc<Vec<Box<dyn Policy>>>,
}

impl PolicyLayer {
    pub fn new(policies: Vec<Box<dyn Policy>>) -> Self {
        Self {
            policies: Arc::new(policies),
        }
    }
}

impl<S> Layer<S> for PolicyLayer {
    type Service = PolicyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PolicyService {
            policies: Arc::clone(&self.policies),
            inner,
        }
    }
}

// The actual service that will process requests
#[derive(Clone)]
pub struct PolicyService<S> {
    policies: Arc<Vec<Box<dyn Policy>>>,
    inner: S,
}

impl<S> Service<Request<Body>> for PolicyService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let policies = Arc::clone(&self.policies);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut current_request = request;

            // Prevent injection of protected bouncer headers
            clear_bouncer_headers(current_request.headers_mut());

            // Process each policy in the chain
            for policy in policies.iter() {
                match policy.process(current_request).await {
                    PolicyResult::Continue(req) => {
                        // Continue to the next policy with the possibly modified request
                        current_request = req;
                    }
                    PolicyResult::Terminate(response) => {
                        // Return early with the response from the policy
                        return Ok(response);
                    }
                }
            }

            // If all policies pass, forward the request to the inner service
            inner.call(current_request).await
        })
    }
}

// Clear all headers that start with x-bouncer-
fn clear_bouncer_headers(headers: &mut axum::http::HeaderMap) {
    let bouncer_headers: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.as_str().to_lowercase().starts_with("x-bouncer-"))
        .map(|(name, _)| name.clone())
        .collect();

    for name in bouncer_headers {
        headers.remove(name);
    }
}

// Extension trait to make it easy to use the policy chain with Axum
pub trait PolicyChainExt {
    fn into_layer(self) -> PolicyLayer;
}

impl PolicyChainExt for Vec<Box<dyn Policy>> {
    fn into_layer(self) -> PolicyLayer {
        PolicyLayer::new(self)
    }
}
