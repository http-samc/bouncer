use std::{collections::HashMap};
use crate::policy::{traits::*};

pub struct PolicyRegistry {
    factories: HashMap<String, Box<dyn Fn(&serde_json::Value) -> Result<Box<dyn Policy>, String>>>,
}

impl PolicyRegistry {
    pub fn new() -> Self {
        Self { factories: HashMap::new() }
    }

    pub fn register_policy<P>(&mut self)
        where P: Policy + 'static 
    {
        self.factories.insert(
            P::policy_id().to_string(),
            Box::new(|config| {
                let parsed_config = serde_json::from_value::<P::Config>(config.clone())
                    .map_err(|e| format!("Failed to parse config: {}", e))?;
                P::new(parsed_config).map(|p| Box::new(p) as Box<dyn Policy>)
            }),
        );
    }

    pub fn build_policy_chain(&self, configs: &[crate::config::PolicyConfig]) 
        -> Result<Vec<Box<dyn Policy>>, String> 
    {
        configs.iter()
            .map(|cfg| self.factories.get(&cfg.provider)
                .ok_or_else(|| format!("Unknown provider {}", cfg.provider))?
                (&cfg.parameters))
            .collect()
    }
}
