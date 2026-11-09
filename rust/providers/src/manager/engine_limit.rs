// BUG-547: the context window a local profile's engine reported for this
// selection. Included by manager.rs.
//
// Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature

impl ProviderManager {
    /// BUG-547: record the context window the profile's engine reported for
    /// the current selection. It survives re-applying the same selection
    /// (session creation applies it twice) and is ignored for any other.
    pub fn set_engine_context_window(&mut self, limit: crate::engine_limits::EngineLimit) {
        if let Some(selection) = self.selected_model_string() {
            tracing::info!(
                selection = %selection,
                tokens = limit.tokens,
                source = %limit.source,
                "BUG-547: context window reported by the engine"
            );
            self.engine_context_window = Some((selection, limit));
        }
    }

    /// BUG-547: the engine-reported context window of the current
    /// selection, with where it came from; `None` when none was reported.
    pub fn engine_context_window(&self) -> Option<&crate::engine_limits::EngineLimit> {
        let (selection, limit) = self.engine_context_window.as_ref()?;
        (self.selected_model_string().as_deref() == Some(selection.as_str())).then_some(limit)
    }
}
