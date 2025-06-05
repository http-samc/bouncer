/// Macro to simplify policy registration for third-party crates
///
/// This macro is designed to be used to register custom policies
/// with the Bouncer server.
///
/// # Example
///
/// ```rust
/// use bouncer::register_policy;
/// use bouncer::policy::traits::{Policy, PolicyFactory, PolicyResult};
/// use axum::body::Body;
/// use axum::http::Request;
/// use async_trait::async_trait;
///
/// pub struct MyCustomPolicy { /* ... */ }
///
/// #[async_trait]
/// impl Policy for MyCustomPolicy {
///     async fn process(&self, request: Request<Body>) -> PolicyResult {
///         // Implementation details...
///         PolicyResult::Continue(request)
///     }
/// }
///
/// pub struct MyCustomPolicyFactory;
///
/// #[async_trait]
/// impl PolicyFactory for MyCustomPolicyFactory {
///     type PolicyType = MyCustomPolicy;
///     type Config = serde_json::Value;
///
///     fn policy_id() -> &'static str {
///         "@mycustom/policy"
///     }
///
///     async fn new(_config: Self::Config) -> Result<Self::PolicyType, String> {
///         Ok(MyCustomPolicy { /* ... */ })
///     }
///
///     fn validate_config(_config: &Self::Config) -> Result<(), String> {
///         Ok(())
///     }
/// }
///
/// // Register the policy so it can be used in Bouncer configurations
/// register_policy!(MyCustomPolicyFactory);
/// ```
#[macro_export]
macro_rules! register_policy {
    ($policy_type:ty) => {
        // For backward compatibility with plugin system
        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn __bouncer_register_policy(
            registry: &mut $crate::policy::registry::PolicyRegistry,
        ) {
            registry.register_policy::<$policy_type>();
        }

        // For the new integrated approach
        $crate::register_custom_policy(|registry| {
            registry.register_policy::<$policy_type>();
        });
    };
}
