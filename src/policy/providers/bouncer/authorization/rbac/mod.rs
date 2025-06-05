pub mod v1;

// Returns policy ID with version
pub fn policy_id_with_version(version: &str) -> &'static str {
    match version {
        "v1" => "@bouncer/authorization/rbac/v1",
        _ => panic!("Unsupported version: {}", version),
    }
}
