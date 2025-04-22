use crate::policy::traits::*;
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::Path;

pub struct PolicyRegistry {
    factories: HashMap<String, Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn Policy>, String>>>,
    // Store loaded libraries to keep them in memory
    #[allow(dead_code)]
    loaded_libraries: Vec<Library>,
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
        }
    }

    pub fn register_policy<F>(&mut self)
    where
        F: PolicyFactory + 'static,
        F::PolicyType: 'static,
    {
        self.factories.insert(
            F::policy_id().to_string(),
            Box::new(|config| {
                let parsed_config = serde_json::from_value::<F::Config>(config.clone())
                    .map_err(|e| format!("Failed to parse config: {}", e))?;
                F::new(parsed_config).map(|p| Box::new(p) as Box<dyn Policy>)
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

    pub fn build_policy_chain(
        &self,
        configs: &[crate::config::PolicyConfig],
    ) -> Result<Vec<Box<dyn Policy>>, String> {
        configs
            .iter()
            .map(|cfg| {
                self.factories
                    .get(&cfg.provider)
                    .ok_or_else(|| format!("Unknown provider {}", cfg.provider))?(
                    &cfg.parameters
                )
            })
            .collect()
    }
}
