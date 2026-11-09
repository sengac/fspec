//! MODEL-014: Codex thinking levels come from the Codex backend
//! `GET /models`.
//!
//! The response lists each model's `supported_reasoning_levels` and
//! `default_reasoning_level`. That is the only source of a Codex model's
//! thinking levels: models.dev and Codex's bundled catalog are never used
//! for Codex (product owner, 2026-11-04, decision 2). An entry resolves into
//! a MODEL-009 [`ReasoningCapability`] and a PROV-152 [`CodexCapabilities`]
//! snapshot. ultra and persistent are dropped first, because fspec has no
//! delegation to back them (product owner, 2026-11-06). A model the response
//! does not list, or an entry fspec cannot use, has no reasoning control,
//! and the reason says why. Other entries are unaffected.
//!
//! The response is fetched once per model selection and kept in memory for
//! that selection only (product owner, 2026-11-06); see [`fetch_for_selection`].
//!
//! Feature: spec/features/codex-thinking-levels-from-codex-models.feature

mod fetch;
mod listing;

#[cfg(feature = "test-support")]
pub use fetch::test_support;
pub use fetch::{fetch, fetch_for_selection, CODEX_BACKEND_BASE_URL, CODEX_MODELS_CLIENT_VERSION};
pub use listing::{fetch_for_listing, ListingFailure, LIST_BUDGET};

use std::collections::HashMap;

use serde::Deserialize;

use super::request_options::{CodexCapabilities, CodexRequestOptions, Effort, ReasoningOptions};
use crate::models::{ReasoningCapability, ReasoningLevel, ReasoningOption};

/// Codex levels fspec does not offer (product owner, 2026-11-06): ultra
/// means Codex's own multi-agent delegation and persistent its
/// disabled-on-the-wire mode, and fspec backs neither.
const NOT_OFFERED: [&str; 2] = ["ultra", "persistent"];

/// The `/models` response, reduced to the fields fspec reads.
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    slug: String,
    /// PROV-176: the name the model selector shows.
    #[serde(default)]
    display_name: Option<String>,
    /// PROV-176: `hide` keeps a model out of the model selector.
    #[serde(default)]
    visibility: Option<String>,
    #[serde(default)]
    supported_reasoning_levels: Vec<LevelPreset>,
    #[serde(default)]
    default_reasoning_level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LevelPreset {
    effort: String,
}

/// What one listed model offers: its efforts (canonical order, without
/// ultra and persistent) and its advertised default.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelReasoning {
    capability: ReasoningCapability,
    default: Option<String>,
}

/// PROV-176: a model the Codex section lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedModel {
    pub slug: String,
    pub display_name: String,
}

/// A fetched Codex `/models` response, by model slug.
#[derive(Debug, Clone)]
pub struct CodexCatalog {
    models: HashMap<String, Result<ModelReasoning, String>>,
    /// PROV-176: the models to list, in response order, without the
    /// entries whose visibility is `hide`.
    listed: Vec<ListedModel>,
}

impl CodexCatalog {
    /// Read a `/models` response body. A body that is not a models list is
    /// an error; an entry fspec cannot use fails only that model.
    pub fn parse(body: &[u8]) -> Result<Self, String> {
        let response: ModelsResponse = serde_json::from_slice(body)
            .map_err(|error| format!("the Codex /models response cannot be read: {error}"))?;
        let mut models = HashMap::new();
        let mut listed = Vec::new();
        for entry in response.models {
            let resolved = resolve_entry(&entry);
            if models.contains_key(&entry.slug) {
                continue;
            }
            if entry.visibility.as_deref() != Some("hide") {
                listed.push(ListedModel {
                    slug: entry.slug.clone(),
                    display_name: entry
                        .display_name
                        .clone()
                        .unwrap_or_else(|| entry.slug.clone()),
                });
            }
            models.insert(entry.slug, resolved);
        }
        Ok(Self { models, listed })
    }

    /// PROV-176: the models the Codex section lists, in response order.
    pub fn listed(&self) -> &[ListedModel] {
        &self.listed
    }

    fn entry(&self, slug: &str) -> Result<&ModelReasoning, String> {
        match self.models.get(slug) {
            None => Err(format!("Codex /models does not list {slug}")),
            Some(Err(reason)) => Err(reason.clone()),
            Some(Ok(reasoning)) => Ok(reasoning),
        }
    }

    /// The thinking levels Codex advertises for `slug`, or why it has none.
    pub fn capability(&self, slug: &str) -> Result<ReasoningCapability, String> {
        self.entry(slug)
            .map(|reasoning| reasoning.capability.clone())
    }

    /// The PROV-152 request-option snapshot for `slug`: its advertised
    /// efforts and default effort. Every other control is not advertised.
    /// `None` when the model has no usable entry (nothing is invented).
    pub fn snapshot(&self, slug: &str) -> Option<CodexCapabilities> {
        let reasoning = self.entry(slug).ok()?;
        let efforts = reasoning
            .capability
            .efforts()
            .iter()
            .map(|effort| Effort::Named(effort.clone()))
            .collect();
        Some(CodexCapabilities {
            efforts: Some(efforts),
            defaults: CodexRequestOptions {
                reasoning: reasoning.default.as_ref().map(|default| ReasoningOptions {
                    effort: Some(Effort::Named(default.clone())),
                    ..ReasoningOptions::default()
                }),
                ..CodexRequestOptions::default()
            },
            ..CodexCapabilities::default()
        })
    }
}

/// One entry's reasoning, or why fspec cannot use it.
fn resolve_entry(entry: &ModelEntry) -> Result<ModelReasoning, String> {
    let slug = &entry.slug;
    let mut efforts = Vec::new();
    for preset in &entry.supported_reasoning_levels {
        ReasoningLevel::effort(&preset.effort).map_err(|_| {
            format!(
                "Codex /models lists {slug} with the level {:?}, which is not a valid effort name",
                preset.effort
            )
        })?;
        if !NOT_OFFERED.contains(&preset.effort.as_str()) && !efforts.contains(&preset.effort) {
            efforts.push(preset.effort.clone());
        }
    }
    if efforts.is_empty() {
        return Err(format!(
            "Codex /models lists no thinking levels fspec offers for {slug}"
        ));
    }
    if let Some(default) = &entry.default_reasoning_level {
        if !efforts.contains(default) {
            return Err(format!(
                "Codex /models gives {slug} the default level {default}, which is not one of \
                 its levels {}",
                efforts.join(", ")
            ));
        }
    }
    let capability = ReasoningCapability::resolve(slug, Some(&[ReasoningOption::Effort(efforts)]))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Codex /models lists no thinking levels for {slug}"))?;
    Ok(ModelReasoning {
        capability,
        default: entry.default_reasoning_level.clone(),
    })
}
