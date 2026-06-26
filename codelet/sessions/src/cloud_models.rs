//! Cloud model catalog population (RPC-073).
//!
//! Ports the model-list sourcing behaviour of the TypeScript
//! `src/tui/services/cloudSectionBuilder.ts` so the Rust model selector shows
//! each configured cloud provider's tool-capable models (sourced from the
//! models.dev catalog) instead of empty rows.
//!
//! The earlier RPC-073 fix wired `list_providers` to
//! `codelet_providers::custom::list_providers_info()`, which returns the
//! canonical built-in providers with EMPTY model lists. This module fills in
//! those models from the `codelet_providers::models` registry, credential-gated
//! and filtered to tool-call-capable, non-deprecated models (newest-first),
//! mirroring the NAPI `models_list_all` parity rules.

use codelet_rpc_types::ModelEntry;

use codelet_providers::models::{Capability, ModelInfo, ModelRegistry, ModelStatus};

/// Map a canonical provider slug (catalog.rs `CANONICAL_PROVIDERS`) to its
/// models.dev provider id. Mirrors TS `mapInternalToProviderId` /
/// `mapModelsDevToRegistryId`: `gemini` ↔ `google`; every other slug is
/// identical between the two namespaces.
pub fn canonical_to_models_dev(canonical_id: &str) -> &str {
    match canonical_id {
        "gemini" => "google",
        other => other,
    }
}

/// Whether a model should appear in the selector — tool-call-capable and not
/// deprecated (mirrors NAPI `is_current_model` + the `toolCall` filter in TS
/// `buildCloudSections`).
fn is_selectable(model: &ModelInfo) -> bool {
    model.tool_call && model.status != Some(ModelStatus::Deprecated)
}

/// Build the tool-capable cloud `ModelEntry` list for a canonical provider from
/// the models.dev registry.
///
/// Returns an empty `Vec` when:
/// - `has_credentials` is `false` (credential-gated, mirrors TS `hasCredentials`),
/// - the provider is absent from models.dev (e.g. `codex`, `github-copilot`), or
/// - the provider has no tool-capable, non-deprecated models.
///
/// Entries are sorted newest-first by release date (NAPI parity).
pub fn cloud_model_entries(
    registry: &ModelRegistry,
    canonical_id: &str,
    has_credentials: bool,
) -> Vec<ModelEntry> {
    if !has_credentials {
        return Vec::new();
    }
    let dev_id = canonical_to_models_dev(canonical_id);
    let models = match registry.list_models(dev_id) {
        Ok(models) => models,
        Err(_) => return Vec::new(),
    };

    let mut selectable: Vec<&ModelInfo> = models.into_iter().filter(|m| is_selectable(m)).collect();

    // Newest-first by release date (undated models sort last).
    selectable.sort_by(|a, b| {
        let date_a = a.release_date.as_deref().unwrap_or("1970-01-01");
        let date_b = b.release_date.as_deref().unwrap_or("1970-01-01");
        date_b.cmp(date_a)
    });

    selectable
        .into_iter()
        .map(|m| ModelEntry {
            id: m.id.clone(),
            display_name: m.name.clone(),
            context_window: m.limit.context,
            supports_reasoning: m.reasoning,
            supports_vision: m.has_capability(Capability::Vision),
            is_custom: false,
        })
        .collect()
}

/// Whether credentials exist for a canonical provider (credentials file → env
/// var → project `.env`), used to gate cloud-model population. Mirrors TS
/// `getProviderConfig().apiKey` presence check.
pub fn provider_has_credentials(canonical_id: &str) -> bool {
    crate::credentials::resolve_credential(canonical_id, None, None)
        .map(|opt| opt.is_some())
        .unwrap_or(false)
}

/// Resolve the SessionHeader `ModelInfo` (wire type) for a session's
/// provider/model pair (TUI-001).
///
/// Mirrors the TypeScript `AgentView.tsx` `rustModelInfo` memo: on a models.dev
/// catalog **hit** the friendly `name`, `reasoning` flag and `Vision`
/// capability come from the registry; on a **miss** (unknown provider/model or
/// no registry attached) it degrades gracefully to the raw `model_id` with both
/// capability flags `false` — matching the prior stub behaviour and the TS
/// fallback path.
///
/// `context_window` / `compaction_threshold` are the Rust-resolved session
/// values (the size badge prefers the threshold; see `size_badge_label` in the
/// fspec-tui header).
pub fn resolve_model_info(
    registry: Option<&ModelRegistry>,
    provider_id: &str,
    model_id: &str,
    context_window: u32,
    compaction_threshold: u32,
) -> codelet_rpc_types::ModelInfo {
    let dev_id = canonical_to_models_dev(provider_id);
    let hit = registry.and_then(|r| r.get_model(dev_id, model_id).ok());
    match hit {
        Some(model) => codelet_rpc_types::ModelInfo {
            display_name: model.name.clone(),
            supports_reasoning: model.reasoning,
            supports_vision: model.has_capability(Capability::Vision),
            context_window,
            compaction_threshold,
        },
        None => codelet_rpc_types::ModelInfo {
            display_name: model_id.to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window,
            compaction_threshold,
        },
    }
}
