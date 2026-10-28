use super::{ExtensionError, ProviderSession};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct PreparedProviderGeneration {
    pub generation: u64,
    pub metadata: HashMap<String, Value>,
}

impl ProviderSession {
    pub fn prepare_compaction_generation(
        &self,
        generation: u64,
        model: &str,
        envelopes: &[String],
    ) -> Result<PreparedProviderGeneration, ExtensionError> {
        let mut prepared = PreparedProviderGeneration {
            generation,
            metadata: HashMap::new(),
        };
        for extension in self.extensions.values() {
            let next = extension.prepare_compaction_generation(generation, model, envelopes)?;
            prepared.generation = next.generation;
            prepared.metadata.extend(next.metadata);
        }
        Ok(prepared)
    }
}
