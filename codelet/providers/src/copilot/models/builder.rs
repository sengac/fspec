//! Pure transformation: parsed Copilot `/models` response → list of [`ModelInfo`] (PROV-056).
//!
//! This module owns the wire-format → domain-type mapping and nothing else.
//! It does **no** IO, depends on no HTTP client, and does not consult any
//! shared state. Given the same input, it always produces the same output.
//!
//! Rule [6]: `ModelInfo` build mapping reads exclusively from the remote
//! response. Nothing is invented, no id-pattern or date heuristics fire.

use super::schema::{CopilotModelEntry, CopilotModelsResponse};
use crate::copilot::constants::{COPILOT_NPM_KEY, COPILOT_PROVIDER_ID};
use crate::models::{LimitInfo, ModelInfo};
use serde_json::Value;
use std::collections::HashMap;

/// Options key under which reasoning-effort tier variants are stored verbatim.
pub const REASONING_VARIANTS_KEY: &str = "reasoning_variants";

/// Options key for the stable `providerID` advertised to the catalog consumer.
pub const PROVIDER_ID_OPTION_KEY: &str = "providerID";

/// Options key for the `npm` AI-SDK package identifier.
pub const NPM_OPTION_KEY: &str = "npm";

/// Pure transformation: parsed `/models` response → list of `ModelInfo`.
///
/// Filters out entries where `model_picker_enabled` is `false`, then maps
/// each survivor through [`build_model_info`]. The relative order of entries
/// in the input is preserved in the output.
#[must_use]
pub fn build_catalog_from_response(response: &CopilotModelsResponse) -> Vec<ModelInfo> {
    response
        .data
        .iter()
        .filter(|entry| entry.model_picker_enabled)
        .map(build_model_info)
        .collect()
}

/// Pure transformation: one `CopilotModelEntry` → one `ModelInfo`.
///
/// Field mapping (PROV-056 rule [6]):
/// - `id`, `name` ← endpoint fields verbatim
/// - `family` ← `capabilities.family` verbatim
/// - `release_date` ← derived from `version` by stripping the `{id}-` prefix;
///   if `version` does not start with the prefix, the version is used verbatim
/// - `limit.context` ← `capabilities.limits.max_context_window_tokens`
/// - `limit.output` ← `capabilities.limits.max_output_tokens`
/// - `tool_call` ← `capabilities.supports.tool_calls`
/// - `reasoning` ← true iff `capabilities.supports.reasoning_effort` is non-empty
/// - `cost` ← `None` (rule [6]: cost is hard-coded to zero / unknown)
/// - `options["providerID"]` ← `"github-copilot"`
/// - `options["npm"]` ← `"@ai-sdk/github-copilot"`
/// - `options["reasoning_variants"]` ← verbatim copy of
///   `capabilities.supports.reasoning_effort` as a JSON array of strings
///   (empty / missing → empty array)
#[must_use]
pub fn build_model_info(entry: &CopilotModelEntry) -> ModelInfo {
    let release_date = derive_release_date(&entry.id, &entry.version);

    let reasoning_variants: &[String] = entry
        .capabilities
        .supports
        .reasoning_effort
        .as_deref()
        .unwrap_or(&[]);

    let mut options: HashMap<String, Value> = HashMap::new();
    options.insert(
        PROVIDER_ID_OPTION_KEY.to_string(),
        Value::String(COPILOT_PROVIDER_ID.to_string()),
    );
    options.insert(
        NPM_OPTION_KEY.to_string(),
        Value::String(COPILOT_NPM_KEY.to_string()),
    );
    options.insert(
        REASONING_VARIANTS_KEY.to_string(),
        Value::Array(
            reasoning_variants
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        ),
    );

    ModelInfo {
        id: entry.id.clone(),
        name: entry.name.clone(),
        family: Some(entry.capabilities.family.clone()),
        release_date: Some(release_date),
        attachment: false,
        reasoning: !reasoning_variants.is_empty(),
        tool_call: entry.capabilities.supports.tool_calls,
        temperature: false,
        interleaved: None,
        modalities: None,
        cost: None,
        limit: LimitInfo {
            context: u32::try_from(entry.capabilities.limits.max_context_window_tokens)
                .unwrap_or(u32::MAX),
            output: u32::try_from(entry.capabilities.limits.max_output_tokens).unwrap_or(u32::MAX),
        },
        status: None,
        experimental: None,
        options,
        headers: HashMap::new(),
    }
}

/// Strip the `{id}-` prefix from the version string. If the version does not
/// start with the prefix, return the raw version unchanged. This is the only
/// release-date logic in the entire module — no calendar table, no per-id
/// special cases.
#[must_use]
pub fn derive_release_date(id: &str, version: &str) -> String {
    let prefix = format!("{id}-");
    version
        .strip_prefix(&prefix)
        .map_or_else(|| version.to_string(), str::to_string)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_entry(
        id: &str,
        version: &str,
        picker_enabled: bool,
        reasoning_effort: Option<Vec<&str>>,
    ) -> CopilotModelEntry {
        let value = json!({
            "id": id,
            "name": id,
            "version": version,
            "model_picker_enabled": picker_enabled,
            "capabilities": {
                "family": "any-family",
                "limits": {
                    "max_context_window_tokens": 100,
                    "max_output_tokens": 50,
                    "max_prompt_tokens": 100
                },
                "supports": {
                    "streaming": true,
                    "tool_calls": true,
                    "vision": false,
                    "reasoning_effort": reasoning_effort
                }
            }
        });
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn picker_disabled_entries_are_filtered_out() {
        let response = CopilotModelsResponse {
            data: vec![
                make_entry("a", "a-2099-01-01", true, None),
                make_entry("b", "b-2099-01-01", false, None),
            ],
        };
        let catalog = build_catalog_from_response(&response);
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].id, "a");
    }

    #[test]
    fn release_date_strips_id_prefix_from_version() {
        let entry = make_entry("foo", "foo-2025-09-15", true, None);
        let info = build_model_info(&entry);
        assert_eq!(info.release_date.as_deref(), Some("2025-09-15"));
    }

    #[test]
    fn release_date_falls_back_to_raw_version_when_prefix_missing() {
        let entry = make_entry("foo", "1.2.3", true, None);
        let info = build_model_info(&entry);
        assert_eq!(info.release_date.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn missing_reasoning_effort_yields_empty_variants() {
        let entry = make_entry("foo", "foo-2099-01-01", true, None);
        let info = build_model_info(&entry);
        let variants = info.options.get(REASONING_VARIANTS_KEY).unwrap();
        assert!(variants.as_array().unwrap().is_empty());
        assert!(!info.reasoning);
    }

    #[test]
    fn empty_reasoning_effort_array_yields_empty_variants() {
        let entry = make_entry("foo", "foo-2099-01-01", true, Some(vec![]));
        let info = build_model_info(&entry);
        let variants = info.options.get(REASONING_VARIANTS_KEY).unwrap();
        assert!(variants.as_array().unwrap().is_empty());
        assert!(!info.reasoning);
    }

    #[test]
    fn reasoning_effort_array_is_copied_verbatim_in_order() {
        let entry = make_entry(
            "foo",
            "foo-2099-01-01",
            true,
            Some(vec!["low", "medium", "high", "xhigh"]),
        );
        let info = build_model_info(&entry);
        let variants: Vec<String> = info
            .options
            .get(REASONING_VARIANTS_KEY)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(variants, vec!["low", "medium", "high", "xhigh"]);
        assert!(info.reasoning);
    }

    #[test]
    fn provider_id_and_npm_are_attached_to_every_entry() {
        let entry = make_entry("foo", "foo-2099-01-01", true, None);
        let info = build_model_info(&entry);
        assert_eq!(
            info.options
                .get(PROVIDER_ID_OPTION_KEY)
                .and_then(|v| v.as_str()),
            Some("github-copilot")
        );
        assert_eq!(
            info.options.get(NPM_OPTION_KEY).and_then(|v| v.as_str()),
            Some("@ai-sdk/github-copilot")
        );
    }
}
