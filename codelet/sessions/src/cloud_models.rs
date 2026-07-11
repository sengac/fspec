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

/// Canonical provider slugs that are legitimately absent from the models.dev
/// catalog. A registry miss for these is EXPECTED, so `cloud_model_entries`
/// returns an empty list silently (no diagnostic warning). Any other slug that
/// misses the registry is a diagnosable slug/key divergence and is logged.
///
/// Only two providers are genuinely absent from models.dev:
/// - `codex`: populated by the separate Codex re-parenting path, not models.dev.
/// - `galadriel`: genuinely not published on models.dev.
///
/// `github-copilot` is intentionally NOT in this set: it IS present in
/// models.dev (key `github-copilot`, canonical slug equals the key), so it
/// resolves normally when credentialed. A future registry miss for it therefore
/// remains diagnosable (warned) rather than silently swallowed.
const KNOWN_ABSENT_FROM_MODELS_DEV: &[&str] = &["codex", "galadriel"];

/// Map a canonical provider slug (catalog.rs `CANONICAL_PROVIDERS`) to its
/// models.dev provider id. Mirrors TS `mapInternalToProviderId` /
/// `mapModelsDevToRegistryId`. Every canonical slug whose models.dev registry
/// key differs must be mapped here; confirmed divergences (verified against the
/// live models.dev catalog for PROV-125): `gemini` → `google`,
/// `together` → `togetherai`, `moonshot` → `moonshotai`. Every other slug is
/// identical between the two namespaces.
pub fn canonical_to_models_dev(canonical_id: &str) -> &str {
    match canonical_id {
        "gemini" => "google",
        "together" => "togetherai",
        "moonshot" => "moonshotai",
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
/// - the provider is absent from models.dev (a known-absent provider such as
///   `codex` or `galadriel` returns empty silently; any other slug that misses
///   is logged via `tracing::warn!` as a diagnosable slug/key divergence before
///   returning empty), or
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
        Err(_) => {
            // A registry miss has two causes (rule 3 vs rule 4): a provider
            // legitimately absent from models.dev (return empty silently) or a
            // diagnosable slug/key divergence (surface it before returning
            // empty, so future divergences don't masquerade as empty rows).
            if !KNOWN_ABSENT_FROM_MODELS_DEV.contains(&canonical_id) {
                tracing::warn!(
                    canonical_id,
                    dev_id,
                    "cloud provider missing from models.dev registry: canonical slug did not resolve to a known models.dev key (possible slug/key divergence); returning empty model list"
                );
            }
            return Vec::new();
        }
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

/// Whether credentials exist for a canonical provider, used to gate cloud-model
/// population. Mirrors TS `cloudSectionBuilder.ts`, which derives a SINGLE
/// `hasCredentials` per section used both to keep the section and to populate
/// it.
///
/// Resolution order:
/// 1. API-key chain (credentials file → env var → project `.env`) via
///    [`crate::credentials::resolve_credential`].
/// 2. PROV-128 OAuth parity: fall back to the SAME
///    [`codelet_providers::ProviderCredentials::detect`] the header-availability
///    flag (`custom::list_providers_info`) reads, honoring OAuth for EVERY
///    OAuth-capable provider (anthropic, codex, github-copilot) — not anthropic
///    only. This makes the population gate and the header flag one credential
///    decision (single source of truth), so an OAuth-only provider is either
///    credentialed-and-populated or dropped — never a credentialed-but-empty
///    "(0 models)" header.
///
/// NOTE: the `openai` → Codex OAuth re-parenting (TS `hasCodexCredentials`
/// override at `cloudSectionBuilder.ts:117-119`) is intentionally out of scope
/// here and handled by PROV-129; this function keeps `openai` gated on its
/// API key.
pub fn provider_has_credentials(canonical_id: &str) -> bool {
    // 1. API-key resolution chain (credentials file → env → project `.env`).
    let has_api_key = crate::credentials::resolve_credential(canonical_id, None, None)
        .map(|opt| opt.is_some())
        .unwrap_or(false);
    if has_api_key {
        return true;
    }

    // 2. PROV-128: OAuth parity via the header-availability flag's own source.
    let creds = codelet_providers::ProviderCredentials::detect();
    match canonical_id {
        "anthropic" => creds.has_claude(),
        "codex" => creds.has_codex(),
        "github-copilot" => creds.has_github_copilot(),
        _ => false,
    }
}

/// PROV-129: The synthetic Codex (ChatGPT) provider slug + label. The `codex`
/// canonical provider already exists as a static, models.dev-absent header
/// (`catalog.rs`); synthesis fills that header in place rather than inserting a
/// new one.
const CODEX_PROVIDER_ID: &str = "codex";
const CODEX_PROVIDER_LABEL: &str = "Codex (ChatGPT)";
const OPENAI_PROVIDER_ID: &str = "openai";

/// PROV-129: build the Codex-supported model list by sourcing the OpenAI cloud
/// catalog (as if credentialed) and filtering it through the Codex allowlist.
///
/// The OpenAI catalog is loaded with `has_credentials = true` unconditionally:
/// the caller ([`synthesize_codex_section`]) has already confirmed Codex
/// credentials, and TS forces `openai.hasCredentials = hasCodexCredentials`
/// (`cloudSectionBuilder.ts:117-119`) so the models flow purely from the Codex
/// gate — independent of `OPENAI_API_KEY`. That is what fixes discrepancy #6
/// (Codex OAuth login previously yielded zero selectable models).
pub fn codex_reparented_models(registry: &ModelRegistry) -> Vec<ModelEntry> {
    let openai_models = cloud_model_entries(registry, OPENAI_PROVIDER_ID, true);
    crate::codex_allowlist::filter_by_codex_allowlist(openai_models)
}

/// PROV-129: synthesize the "Codex (ChatGPT)" section by re-parenting the
/// OpenAI cloud models when Codex credentials are present. Ports TS
/// `cloudSectionBuilder.ts` `extractCodexSection` (:191-237) + the cloud-group
/// assembly (:150-155).
///
/// Behaviour (TS parity):
/// * With NO Codex credentials, `sections` is returned unchanged — the standalone
///   OpenAI section keeps its existing (API-key-gated) behaviour and the empty
///   `codex` header is left for the PROV-127 drop-empty filter to remove.
/// * With Codex credentials but no allowlisted models to re-parent, `sections`
///   is likewise returned unchanged (nothing to synthesize; the empty `codex`
///   header is dropped downstream and a legitimate standalone OpenAI section is
///   preserved).
/// * With Codex credentials AND >=1 allowlisted OpenAI model, the standalone
///   `openai` section and the static empty `codex` header are removed and a
///   freshly-built Codex section (allowlist-filtered models) is PREPENDED so it
///   LEADS the cloud group — mirroring TS `[codexSection, ...remaining]`
///   (PROV-130 ordering parity). The OpenAI models are moved, not duplicated.
///
/// The Codex gate reuses the PROV-128 unified credential predicate
/// [`provider_has_credentials`] (`"codex"`), which honours Codex OAuth and the
/// Codex API key alike — the single source of truth shared with the header
/// availability flag.
pub fn synthesize_codex_section(
    mut sections: Vec<codelet_rpc_types::ProviderInfo>,
    registry: &ModelRegistry,
) -> Vec<codelet_rpc_types::ProviderInfo> {
    if !provider_has_credentials(CODEX_PROVIDER_ID) {
        return sections;
    }

    let codex_models = codex_reparented_models(registry);
    if codex_models.is_empty() {
        return sections;
    }

    // Re-parent (TS `extractCodexSection`): drop the standalone OpenAI cloud
    // section (its models now live under Codex) AND the static, models.dev-absent
    // `codex` header, so the freshly-built Codex section can LEAD the cloud group
    // (PROV-130) without duplicating the header. A local-server OpenAI *profile*
    // section (`profile_name` Some(..)) is never a cloud section and is preserved.
    sections.retain(|s| {
        !(s.profile_name.is_none() && (s.key == OPENAI_PROVIDER_ID || s.key == CODEX_PROVIDER_ID))
    });

    // PROV-130: the synthesized Codex section LEADS the cloud group — TS
    // `cloudSectionBuilder.ts:150-155` pushes `codexSection` first, then the
    // remaining credentialed providers (relative order preserved).
    let codex = crate::profile_sections::cloud_provider_info(
        CODEX_PROVIDER_ID,
        CODEX_PROVIDER_LABEL,
        codex_models,
    );
    let mut result = Vec::with_capacity(sections.len() + 1);
    result.push(codex);
    result.extend(sections);
    result
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
