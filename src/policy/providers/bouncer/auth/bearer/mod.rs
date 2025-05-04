pub mod v1;

// Export nothing by default - users must specify a version
// No more default exports or backward compatibility layer

// Returns policy ID with version - now requires a version
pub fn policy_id_with_version(version: &str) -> &'static str {
    match version {
        "v1" => "@bouncer/auth/bearer/v1",
        _ => panic!("Unsupported version: {}", version)
    }
}