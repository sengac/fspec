//! PROV-129 — Codex model allowlist (TS parity with
//! `src/tui/services/codexAllowlistService.ts`).
//!
//! Feature: spec/features/codex-section-synthesis.feature
//!
//! When Codex (ChatGPT) credentials are present the model selector re-parents
//! the OpenAI cloud catalog under a synthetic "Codex (ChatGPT)" section
//! (see [`crate::cloud_models::synthesize_codex_section`]). This module filters
//! that catalog down to the Codex-supported model slugs, exactly mirroring the
//! TS `matchesCodexAllowlist` / `filterByCodexAllowlist`:
//!
//! * Only entries whose `visibility` equals `"list"` can match.
//! * A model matches by an EXACT slug equality, or by a `slug-<date>` variant
//!   where `<date>` is a `YYYY-MM-DD` or `YYYYMMDD` suffix (not a different
//!   model family such as `-mini` / `-nano`).
//! * Matched models are sorted by ascending allowlist `priority`.
//!
//! The bundled default allowlist is the SAME JSON the TS build ships
//! (`src/tui/data/codex-models.json`), embedded via `include_str!` so there is
//! a single source of truth. A user override at `~/.fspec/codex-models.json`
//! (resolved via `FSPEC_USER_DIR`/`HOME`) takes precedence when it exists and
//! is non-empty. All failures degrade gracefully — a load error yields the
//! unfiltered model list, matching the TS try/catch.

use codelet_rpc_types::ModelEntry;
use serde::Deserialize;

/// Bundled default Codex allowlist — shared verbatim with the TS build so the
/// Rust and Ink selectors filter identically.
const BUNDLED_ALLOWLIST_JSON: &str = include_str!("../../../src/tui/data/codex-models.json");

/// A single Codex allowlist entry (`{ slug, visibility, priority }`). Mirrors
/// the TS `CodexModelEntry`.
#[derive(Debug, Clone, Deserialize)]
pub struct CodexModelEntry {
    /// Model slug (exact id or the stem of a dated variant).
    pub slug: String,
    /// `"list"` (selectable) or `"hide"` (excluded even when catalogued).
    pub visibility: String,
    /// Sort priority (lower = higher priority).
    pub priority: i64,
}

/// Wrapper for the allowlist JSON document (`{ version, models: [...] }`).
#[derive(Debug, Deserialize)]
struct CodexAllowlistConfig {
    #[serde(default)]
    models: Vec<CodexModelEntry>,
}

/// Load the Codex allowlist. Tries the user override
/// `~/.fspec/codex-models.json` first, falling back to the bundled default.
/// Mirrors TS `loadCodexAllowlist`.
pub fn load_codex_allowlist() -> Vec<CodexModelEntry> {
    if let Some(user) = load_user_allowlist() {
        return user;
    }
    parse_allowlist(BUNDLED_ALLOWLIST_JSON).unwrap_or_default()
}

/// Read + parse the user override, returning `None` on a missing / malformed /
/// empty file so the caller falls back to the bundled default.
fn load_user_allowlist() -> Option<Vec<CodexModelEntry>> {
    let dir = crate::profile_sections::fspec_user_dir()?;
    let content = std::fs::read_to_string(dir.join("codex-models.json")).ok()?;
    let models = parse_allowlist(&content)?;
    if models.is_empty() {
        None
    } else {
        Some(models)
    }
}

/// Parse an allowlist JSON document into its `models` list, or `None` on error.
fn parse_allowlist(source: &str) -> Option<Vec<CodexModelEntry>> {
    serde_json::from_str::<CodexAllowlistConfig>(source)
        .ok()
        .map(|config| config.models)
}

/// Whether `suffix` is a recognised date suffix: `YYYY-MM-DD` or `YYYYMMDD`
/// (mirrors the TS regexes `/^\d{4}-\d{2}-\d{2}$/` and `/^\d{8}$/`).
fn is_date_suffix(suffix: &str) -> bool {
    let bytes = suffix.as_bytes();
    // YYYY-MM-DD
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        let ok = bytes.iter().enumerate().all(|(i, &b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
        if ok {
            return true;
        }
    }
    // YYYYMMDD
    bytes.len() == 8 && bytes.iter().all(u8::is_ascii_digit)
}

/// Whether a `list`-visible allowlist entry matches `model_id` (exact slug or a
/// dated variant `slug-<date>`). Mirrors TS `matchesCodexAllowlist`.
pub fn matches_codex_allowlist(model_id: &str, allowlist: &[CodexModelEntry]) -> bool {
    allowlist.iter().any(|entry| entry_matches(entry, model_id))
}

/// Whether one entry matches `model_id`. Hidden entries never match.
fn entry_matches(entry: &CodexModelEntry, model_id: &str) -> bool {
    if entry.visibility != "list" {
        return false;
    }
    if model_id == entry.slug {
        return true;
    }
    match model_id.strip_prefix(&entry.slug) {
        Some(rest) => rest.strip_prefix('-').map(is_date_suffix).unwrap_or(false),
        None => false,
    }
}

/// The allowlist priority of `model_id` (lower = higher priority), or
/// `i64::MAX` when it matches nothing. Mirrors TS `getModelPriority`.
fn model_priority(model_id: &str, allowlist: &[CodexModelEntry]) -> i64 {
    allowlist
        .iter()
        .find(|entry| entry_matches(entry, model_id))
        .map(|entry| entry.priority)
        .unwrap_or(i64::MAX)
}

/// Filter `models` to the Codex-supported, `list`-visible entries and sort them
/// by ascending allowlist priority. Mirrors TS `filterByCodexAllowlist`.
///
/// When the allowlist is empty (bundled default missing AND no override) the
/// models pass through unfiltered — matching the TS branch that only filters
/// when `allowlist.length > 0`.
pub fn filter_by_codex_allowlist(models: Vec<ModelEntry>) -> Vec<ModelEntry> {
    let allowlist = load_codex_allowlist();
    if allowlist.is_empty() {
        return models;
    }
    let mut filtered: Vec<ModelEntry> = models
        .into_iter()
        .filter(|m| matches_codex_allowlist(&m.id, &allowlist))
        .collect();
    filtered.sort_by_key(|m| model_priority(&m.id, &allowlist));
    filtered
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn model(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            context_window: 128_000,
            supports_reasoning: false,
            supports_vision: false,
            is_custom: false,
        }
    }

    #[test]
    fn bundled_allowlist_parses_and_is_non_empty() {
        let allowlist = parse_allowlist(BUNDLED_ALLOWLIST_JSON).expect("bundled allowlist parses");
        assert!(
            !allowlist.is_empty(),
            "the bundled codex-models.json must contain entries",
        );
        assert!(
            allowlist
                .iter()
                .any(|e| e.slug == "gpt-5.4" && e.visibility == "list"),
            "bundled allowlist must list gpt-5.4",
        );
    }

    #[test]
    fn exact_and_dated_variants_match_only_for_list_visibility() {
        let allowlist = parse_allowlist(BUNDLED_ALLOWLIST_JSON).unwrap();
        // exact list slug matches
        assert!(matches_codex_allowlist("gpt-5.4", &allowlist));
        // dated variant of a list slug matches
        assert!(matches_codex_allowlist(
            "gpt-5.2-codex-2026-03-01",
            &allowlist
        ));
        assert!(matches_codex_allowlist(
            "gpt-5.2-codex-20260301",
            &allowlist
        ));
        // hidden slug never matches
        assert!(!matches_codex_allowlist("gpt-5.1-codex", &allowlist));
        assert!(!matches_codex_allowlist("gpt-5", &allowlist));
        // unrelated family suffix does not match
        assert!(!matches_codex_allowlist("gpt-5-mini", &allowlist));
    }

    #[test]
    fn filter_keeps_allowlisted_and_orders_by_priority() {
        let filtered = filter_by_codex_allowlist(vec![
            model("gpt-5.2-codex"),
            model("gpt-4o"),
            model("gpt-5.4"),
            model("gpt-5-mini"),
            model("gpt-5.1-codex"),
        ]);
        let ids: Vec<&str> = filtered.iter().map(|m| m.id.as_str()).collect();
        // gpt-5.4 (priority 0) precedes gpt-5.2-codex (priority 3); the rest drop.
        assert_eq!(ids, vec!["gpt-5.4", "gpt-5.2-codex"]);
    }
}
