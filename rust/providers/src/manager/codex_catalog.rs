// MODEL-014: the Codex /models response fetched for this selection, kept on
// the ProviderManager in memory only. Included by manager.rs.

impl ProviderManager {
    /// MODEL-014: fetch Codex /models for this selection when the selected
    /// provider is Codex (product owner, 2026-11-06: once per model
    /// selection; session create, resume and model switch each build a new
    /// manager and call this). A failed fetch is kept as the reason Codex
    /// models have no reasoning control until the next selection retries.
    pub async fn fetch_codex_catalog(&mut self) {
        if self.current_provider != ProviderType::Codex {
            return;
        }
        let catalog = crate::codex::models_catalog::fetch_for_selection().await;
        match &catalog {
            Ok(_) => tracing::info!("MODEL-014: fetched Codex /models for this selection"),
            Err(reason) => tracing::warn!(
                "MODEL-014: Codex models have no reasoning control for this selection: {reason}"
            ),
        }
        self.codex_catalog = Some(catalog);
    }

    /// MODEL-014: record a fetched (or failed) Codex /models response.
    pub fn set_codex_catalog(
        &mut self,
        catalog: Result<crate::codex::models_catalog::CodexCatalog, String>,
    ) {
        self.codex_catalog = Some(catalog);
    }

    /// MODEL-014: the Codex /models response for this selection; `None`
    /// when none was fetched (only the async model resolution fetches).
    pub fn codex_catalog(
        &self,
    ) -> Option<&Result<crate::codex::models_catalog::CodexCatalog, String>> {
        self.codex_catalog.as_ref()
    }
}
