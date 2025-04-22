/// Macro to simplify policy registration for third-party crates
///
/// This macro is designed to be used in external crates that implement custom policies
/// for the Bouncer server.
///
/// # Example
///
/// ```rust
/// use bouncer_core::register_policy;
///
/// pub struct MyCustomPolicy { /* ... */ }
///
/// impl Policy for MyCustomPolicy {
///     // Implementation details...
/// }
///
/// // Register the policy so it can be used in Bouncer configurations
/// register_policy!(MyCustomPolicy);
/// ```
#[macro_export]
macro_rules! register_policy {
    ($policy_type:ty) => {
        // This needs to be in the root of the crate to be visible
        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn __bouncer_register_policy(registry: &mut $crate::policy::registry::PolicyRegistry) {
            registry.register_policy::<$policy_type>();
        }
    };
} 