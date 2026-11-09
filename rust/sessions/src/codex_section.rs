//! PROV-176: the Codex (ChatGPT) section lists only the models Codex
//! `/models` returns.
//!
//! The list is fetched (MODEL-014's ChatGPT-signed `/models` request with the
//! pinned client version) every time the provider list is built. BUG-577:
//! the list waits at most `models_catalog::LIST_BUDGET` for the answer and
//! joins a fetch already in flight, so it always arrives well under the RPC
//! deadline; an answer that lists no models says so. Entries with
//! visibility `hide` are not listed. Nothing from models.dev is substituted:
//! a fetch that fails, or Codex signed in with an API key only, shows the
//! section with no models and the reason. The models.dev OpenAI catalog is
//! no longer re-parented under Codex (PROV-129), and the openai provider
//! keeps PROV-146: it has no cloud section (product owner, 2026-11-06). A
//! listed model's context window and vision flag come from its models.dev
//! OpenAI entry when there is one, the entry its limits come from at
//! selection; its reasoning flag comes from what `/models` advertises.
//!
//! Feature: spec/features/the-codex-section-lists-only-the-models-codex-models-returns.feature

use codelet_providers::codex::models_catalog::{self, CodexCatalog, ListedModel};
use codelet_providers::models::{Capability, ModelRegistry};
use codelet_rpc_types::{ModelEntry, ProviderInfo};

const CODEX_PROVIDER_ID: &str = "codex";
const CODEX_PROVIDER_LABEL: &str = "Codex (ChatGPT)";
/// The models.dev provider a Codex model's limits come from.
const OPENAI_PROVIDER_ID: &str = "openai";

/// The Codex section when Codex has credentials, leading the cloud group
/// (PROV-130) in place of the static, models.dev-absent `codex` header.
/// Without Codex credentials `sections` is returned unchanged.
pub fn synthesize_codex_section(
    mut sections: Vec<ProviderInfo>,
    registry: Option<&ModelRegistry>,
) -> Vec<ProviderInfo> {
    if !crate::cloud_models::provider_has_credentials(CODEX_PROVIDER_ID) {
        return sections;
    }
    // BUG-577: the shared, budgeted fetch, worded for the model selector.
    let listing = models_catalog::fetch_for_listing().map_err(|failure| failure.reason());
    let codex = codex_section(registry, listing);
    sections
        .retain(|section| !(section.profile_name.is_none() && section.key == CODEX_PROVIDER_ID));
    let mut result = Vec::with_capacity(sections.len() + 1);
    result.push(codex);
    result.extend(sections);
    result
}

/// BUG-577: the reason shown when `/models` answers with no models to list.
pub const NO_MODELS_LISTED: &str = "Codex /models listed no models";

/// The Codex section for a `/models` listing, or for why there is none.
/// BUG-577: an answer with nothing to list carries [`NO_MODELS_LISTED`], so
/// the section is kept and startup never mistakes it for a missing provider.
pub fn codex_section(
    registry: Option<&ModelRegistry>,
    listing: Result<CodexCatalog, String>,
) -> ProviderInfo {
    let listing = listing.and_then(|catalog| {
        if catalog.listed().is_empty() {
            Err(NO_MODELS_LISTED.to_string())
        } else {
            Ok(catalog)
        }
    });
    match listing {
        Ok(catalog) => {
            let models = catalog
                .listed()
                .iter()
                .map(|listed| model_entry(registry, &catalog, listed))
                .collect();
            crate::profile_sections::cloud_provider_info(
                CODEX_PROVIDER_ID,
                CODEX_PROVIDER_LABEL,
                models,
            )
        }
        Err(reason) => ProviderInfo {
            unavailable_reason: Some(reason),
            ..crate::profile_sections::cloud_provider_info(
                CODEX_PROVIDER_ID,
                CODEX_PROVIDER_LABEL,
                Vec::new(),
            )
        },
    }
}

fn model_entry(
    registry: Option<&ModelRegistry>,
    catalog: &CodexCatalog,
    listed: &ListedModel,
) -> ModelEntry {
    let known = registry
        .and_then(|registry| registry.list_models(OPENAI_PROVIDER_ID).ok())
        .and_then(|models| models.into_iter().find(|model| model.id == listed.slug));
    ModelEntry {
        id: listed.slug.clone(),
        display_name: listed.display_name.clone(),
        // PROV-169: models.dev reports an unknown context as 0.
        context_window: known.and_then(|model| model.limit.context_window()),
        supports_reasoning: catalog.capability(&listed.slug).is_ok(),
        supports_vision: known.is_some_and(|model| model.has_capability(Capability::Vision)),
        is_custom: false,
    }
}

/// PROV-176: a Codex model whose limits models.dev does not know is listed
/// but refused at selection (session create, resume and model switch),
/// naming the missing limits; no limit is invented or derived from
/// `/models` (product owner, 2026-11-06). Other selections are untouched.
pub(crate) fn refuse_codex_model_without_limits(
    model: &str,
    limits: &crate::model_resolution::ResolvedModelLimits,
) -> Result<(), String> {
    let Some(slug) = model.strip_prefix("codex/") else {
        return Ok(());
    };
    let missing = [
        ("context window", limits.context_window.is_none()),
        ("output limit", limits.max_output_tokens.is_none()),
    ]
    .into_iter()
    .filter_map(|(name, missing)| missing.then_some(name))
    .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{model} cannot be selected: models.dev has no {} for {slug}, and fspec does not \
         invent model limits",
        missing.join(" and no ")
    ))
}
