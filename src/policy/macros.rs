/// Macro to simplify policy registration for third-party crates
///
/// This macro is designed to be used to register custom policies
/// with the Bouncer server.
///
/// # Example
///
/// ```rust
/// use bouncer::register_policy;
/// use bouncer::{Policy, PolicyFactory, PolicyResult};
///
/// pub struct MyCustomPolicy { /* ... */ }
///
/// impl Policy for MyCustomPolicy {
///     // Implementation details...
/// }
///
/// pub struct MyCustomPolicyFactory;
///
/// impl PolicyFactory for MyCustomPolicyFactory {
///     // Implementation details...
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
        pub extern "C" fn __bouncer_register_policy(registry: &mut $crate::policy::registry::PolicyRegistry) {
            registry.register_policy::<$policy_type>();
        }
        
        // For the new integrated approach
        $crate::register_custom_policy(|registry| {
            registry.register_policy::<$policy_type>();
        });
    };
} 