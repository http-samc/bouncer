pub mod v1;
pub mod v1_managed;

// Export nothing by default - users must specify a version
// No more default exports or backward compatibility layer

// Returns policy ID with version - now requires a version
pub fn policy_id_with_version(version: &str) -> &'static str {
    match version {
        "v1" => "@bouncer/auth/bearer/v1",
        "v1-managed" => "@bouncer/auth/bearer/v1-managed",
        _ => panic!("Unsupported version: {}", version)
    }
} 