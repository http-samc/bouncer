use crate::config::PolicyConfig;
use crate::policy::routes::PolicyRouter;
use crate::policy::traits::{Policy, PolicyFactory};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::Path;
use tracing;

pub struct PolicyRegistry {
    factories: HashMap<String, Box<dyn Fn(&serde_json::Value) -> futures::future::BoxFuture<'static, Result<Box<dyn Policy>, String>> + Send + Sync>>,
    // Store loaded libraries to keep them in memory
    #[allow(dead_code)]
    loaded_libraries: Vec<Library>,
    // Store policy routes
    // policy_router: PolicyRouter,
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            loaded_libraries: Vec::new(),
            // policy_router: PolicyRouter::new(),
        }
    }

    pub fn register_policy<F>(&mut self)
    where
        F: PolicyFactory + 'static,
        F::PolicyType: 'static,
    {
        let policy_id = F::policy_id().to_string();
        tracing::debug!("Registering policy: {}", policy_id);

        self.factories.insert(
            policy_id,
            Box::new(move |config| {
                let parsed_config = match serde_json::from_value::<F::Config>(config.clone()) {
                    Ok(config) => config,
                    Err(e) => return Box::pin(futures::future::ready(Err(format!("Failed to parse config: {}", e)))),
                };

                Box::pin(async move {
                    match F::new(parsed_config).await {
                        Ok(policy) => Ok(Box::new(policy) as Box<dyn Policy>),
                        Err(e) => Err(e),
                    }
                })
            }),
        );
    }

    /// Load a policy from a dynamic library
    ///
    /// This function loads a dynamic library containing a policy implementation
    /// and registers it with the policy registry.
    pub fn load_policy_from_library<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        // Load the dynamic library
        let lib = unsafe {
            Library::new(path.as_ref()).map_err(|e| format!("Failed to load library: {}", e))?
        };

        // Find and call the registration function
        let register_fn: Symbol<unsafe extern "C" fn(&mut PolicyRegistry)> = unsafe {
            lib.get(b"__bouncer_register_policy")
                .map_err(|e| format!("Failed to find registration function: {}", e))?
        };

        // Call the registration function
        unsafe { register_fn(self) };

        // Store the library to keep it loaded
        self.loaded_libraries.push(lib);

        Ok(())
    }

    /// Load all policy plugins from a directory
    ///
    /// This function scans a directory for dynamic libraries and attempts to load
    /// each one as a policy plugin.
    pub fn load_policies_from_directory<P: AsRef<Path>>(
        &mut self,
        dir_path: P,
    ) -> Result<(), String> {
        let dir_path = dir_path.as_ref();
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read plugin directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Only try to load files with the appropriate extension for the platform
            let extension = if cfg!(target_os = "windows") {
                "dll"
            } else if cfg!(target_os = "macos") {
                "dylib"
            } else {
                "so"
            };

            if path.is_file() && path.extension().is_some_and(|ext| ext == extension) {
                if let Err(e) = self.load_policy_from_library(&path) {
                    tracing::warn!("Failed to load policy from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    // Split a policy provider identifier into parts
    // For example, "@bouncer/auth/bearer/v1" -> ("@bouncer/auth/bearer", "v1")
    // fn split_policy_provider(provider: &str) -> Result<(String, String), String> {
    //     let parts: Vec<&str> = provider.split('/').collect();
    //     if parts.len() < 4 || !parts.last().unwrap().starts_with('v') {
    //         return Err(format!("Invalid policy ID: {}. All policies must specify a version (e.g., @provider/category/name/v1)", provider));
    //     }

    //     let version = parts.last().unwrap().to_string();
    //     let base_provider = parts[..parts.len() - 1].join("/");
    //     Ok((base_provider, version))
    // }

    /// Build a policy chain from a list of policy configurations
    pub async fn build_policy_chain(
        &self,
        config: &[PolicyConfig],
    ) -> Result<(Vec<Box<dyn Policy>>, PolicyRouter), String> {
        let mut policy_chain = Vec::new();
        let mut policy_router = PolicyRouter::new();

        for policy_config in config {
            let factory = self.factories
                .get(&policy_config.provider)
                .ok_or_else(|| {
                    format!(
                        "Policy not found for provider ID: {}",
                        policy_config.provider
                    )
                })?;

            let policy = factory(&policy_config.parameters).await?;

            // Register routes for all policies
            let routes = policy.register_routes();
            if !routes.is_empty() {
                let base_path = format!(
                    "/_admin/{}/{}/{}/{}",
                    policy.provider(),
                    policy.category(),
                    policy.name(),
                    policy.version()
                );
                policy_router.register_routes(routes, &base_path);
            }

            // Only add to policy chain if the policy processes requests
            if policy.processes_requests() {
                policy_chain.push(policy);
            }
        }

        Ok((policy_chain, policy_router))
    }
}
