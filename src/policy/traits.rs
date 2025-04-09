use axum::{
  http::{Request, Response},
};
use async_trait::async_trait;
use serde::Deserialize;

pub enum PolicyResult {
  Continue(Request<axum::body::Body>),
  Terminate(Response<axum::body::Body>),
}

#[async_trait]
pub trait Policy: Send + Sync + 'static {
  type Config: for<'de> Deserialize<'de> + Send + Sync + 'static;

  fn new(config: Self::Config) -> Result<Self, String> where Self: Sized;

  async fn process(&self, request: Request<axum::body::Body>) -> PolicyResult;

  fn validate_config(config: &Self::Config) -> Result<(), String>;
}
