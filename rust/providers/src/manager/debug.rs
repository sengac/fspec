// ProviderManager Debug (moved out of manager.rs to keep it under the
// REFAC-028 300-line limit). Included by manager.rs.

impl std::fmt::Debug for ProviderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderManager")
            .field("current_provider", &self.current_provider)
            .field("selected_model", &self.selected_model)
            .field("has_model_registry", &self.model_registry.is_some())
            .finish()
    }
}
