//! Helpers extracted from `model_selector_dialog.rs` so the parent
//! file stays under the 300-LoC budget.
//!
//! Feature: spec/features/rpc022-model-selector-dialog.feature
//! Feature: spec/features/rpc027-model-confirm-dialogs.feature
//!
//! RPC-027 update: `DialogBody` adapter removed; rows now flow into
//! the shared `dialog_theme` renderer instead of `tui_popup`.

use codelet_rpc_types::ProviderInfo;

/// One displayable row in the dialog. Provider headers are not
/// selectable (`selectable = false`); model rows are.
#[derive(Debug, Clone)]
pub(super) struct ModelSelectorRow {
    /// Render text for the row (without the leading `▸ ` marker).
    pub(super) label: String,
    /// Capability/context badge suffix (e.g. `" [R] [V] [200k]"`),
    /// rendered DIM on unselected rows.
    pub(super) badges: String,
    /// True for model rows (Enter emits ModelSelected); false for
    /// provider headers.
    pub(super) selectable: bool,
    /// Provider key — populated only when `selectable` is true.
    pub(super) provider_key: String,
    /// Model id — populated only when `selectable` is true.
    pub(super) model_id: String,
}

/// Flatten a `[ProviderInfo]` list into a flat `[ModelSelectorRow]`
/// projection: a `▼ provider_name` header followed by each model row.
pub(super) fn build_rows(providers: &[ProviderInfo]) -> Vec<ModelSelectorRow> {
    let mut rows = Vec::with_capacity(providers.len() * 4);
    for provider in providers {
        rows.push(ModelSelectorRow {
            label: format!(
                "▼ {} ({} models)",
                provider.display_name,
                provider.models.len()
            ),
            badges: String::new(),
            selectable: false,
            provider_key: String::new(),
            model_id: String::new(),
        });
        for model in &provider.models {
            let label = model.display_name.clone();
            let mut badges = String::new();
            if model.supports_reasoning {
                badges.push_str(" [R]");
            }
            if model.supports_vision {
                badges.push_str(" [V]");
            }
            if model.context_window > 0 {
                let cw = if model.context_window >= 1_000 {
                    format!("{}k", model.context_window / 1_000)
                } else {
                    model.context_window.to_string()
                };
                badges.push_str(&format!(" [{cw}]"));
            }
            rows.push(ModelSelectorRow {
                label,
                badges,
                selectable: true,
                provider_key: provider.key.clone(),
                model_id: model.id.clone(),
            });
        }
    }
    rows
}
