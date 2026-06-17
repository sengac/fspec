//! `suggest-dependencies` — Rust port of `suggestDependencies` in
//! `src/commands/suggest-dependencies.ts` (RPC-309).
//!
//! Read-only command: auto-suggests dependency relationships based on work
//! unit metadata. Loads `spec/work-units.json` via [`ensure_work_units_file`]
//! (auto-creates the canonical empty store on ENOENT, escalates malformed
//! JSON via [`FspecCoreError::ParseJson`] with `file = "work-units.json"`).
//!
//! ## Rules (parity with the TS source)
//!
//! 1. **Build/Test pairs** (HIGH, evaluated first): a `Test X` work unit
//!    `dependsOn` a `Build X` work unit when the build title contains the test
//!    target. Marks the pair in `specific_matches`.
//! 2. **Infrastructure-before-features** (HIGH): a feature work unit (title
//!    starts with `add`/`create`/`implement`/`build`) `dependsOn` a same-prefix
//!    infra work unit (title contains `schema`/`migration`/`database schema`/
//!    `setup`/`infrastructure`). Marks the pair in `specific_matches`.
//! 3. **Sequential IDs** (MEDIUM, fallback): within a prefix, units sorted by
//!    numeric ID suffix, each `dependsOn` its predecessor, skipped when a
//!    specific pattern already matched the pair OR an existing relationship
//!    exists.
//! 4. **Existing relationships excluded**: never suggest when `from` already
//!    lists `to` in `dependsOn` or `blockedBy`.
//! 5. **Circular suggestions filtered**: when a reverse suggestion exists for
//!    the same pair, only the one whose `from < to` (lexicographic) is kept.
//!
//! The JSDoc-documented `same epic -> relatesTo` rule is NOT implemented in
//! the TS source; the Rust port mirrors TS and emits only `dependsOn`
//! suggestions.
//!
//! Output: `output='json'` returns pretty-printed JSON
//! `{suggestions:[{from,to,type,reason,confidence}]}` in declaration order;
//! default text prints a numbered summary, or `No dependency suggestions
//! found.` when empty.
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge
//! both call this single function — no inline rendering elsewhere.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

/// A single dependency suggestion. Field declaration order MUST be
/// `from, to, type, reason, confidence` for byte-parity with the TS JSON
/// output (`r#type` is renamed to `type`).
#[derive(Debug, Clone, Serialize)]
struct DependencySuggestion {
    from: String,
    to: String,
    #[serde(rename = "type")]
    r#type: String,
    reason: String,
    confidence: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SuggestDependenciesArgs {
    /// `text` (default) or `json`.
    #[serde(default)]
    output: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: SuggestDependenciesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "suggest-dependencies",
            reason: format!("failed to parse args: {e}"),
        })?;

    let data = ensure_work_units_file(project_root)?;
    let suggestions = compute_suggestions(&data);

    let json_mode = args.output.as_deref() == Some("json");
    if json_mode {
        // Serialize a dedicated wrapper struct (NOT a `serde_json::Value`)
        // so the suggestion field order from,to,type,reason,confidence is
        // preserved verbatim regardless of serde_json's `preserve_order`
        // feature flag.
        #[derive(Serialize)]
        struct Output<'a> {
            suggestions: &'a [DependencySuggestion],
        }
        serde_json::to_string_pretty(&Output {
            suggestions: &suggestions,
        })
        .map_err(|e| FspecCoreError::InvalidArgs {
            command: "suggest-dependencies",
            reason: format!("failed to serialize suggestions: {e}"),
        })
    } else {
        Ok(render_text(&suggestions))
    }
}

/// Compute the dependency suggestions for `data`, mirroring the TS rule order
/// (build/test → infrastructure → sequential fallback → circular filter).
fn compute_suggestions(data: &WorkUnitsData) -> Vec<DependencySuggestion> {
    // `Object.values(data.workUnits)` — insertion order preserved by IndexMap.
    let work_units: Vec<&WorkUnit> = data.work_units.values().collect();

    let mut suggestions: Vec<DependencySuggestion> = Vec::new();
    // Track IDs that have specific pattern matches to avoid generic sequential
    // suggestions for the same pair ("<from>-><to>").
    let mut specific_matches: std::collections::HashSet<String> = std::collections::HashSet::new();

    // ── Rule 3 (TS): Build/Test pairs (HIGH, evaluated first) ──
    for &wu in &work_units {
        let title = wu.title.to_lowercase();
        if let Some(test_target) = title.strip_prefix("test ") {
            let test_target = test_target.trim_start();
            for &candidate in &work_units {
                if candidate.id == wu.id {
                    continue;
                }
                let candidate_title = candidate.title.to_lowercase();
                if candidate_title.starts_with("build ") && candidate_title.contains(test_target) {
                    if relationship_contains(wu, "dependsOn", &candidate.id)
                        || relationship_contains(wu, "blockedBy", &candidate.id)
                    {
                        continue;
                    }
                    suggestions.push(DependencySuggestion {
                        from: wu.id.clone(),
                        to: candidate.id.clone(),
                        r#type: "dependsOn".to_string(),
                        reason: format!(
                            "test work depends on build work: \"{}\" depends on \"{}\"",
                            wu.title, candidate.title
                        ),
                        confidence: "high".to_string(),
                    });
                    specific_matches.insert(format!("{}->{}", wu.id, candidate.id));
                }
            }
        }
    }

    // ── Rule 4 (TS): Infrastructure before features (HIGH) ──
    const INFRA_KEYWORDS: &[&str] = &[
        "schema",
        "migration",
        "database schema",
        "setup",
        "infrastructure",
    ];
    const FEATURE_KEYWORDS: &[&str] = &["add", "create", "implement", "build"];

    for &feature_wu in &work_units {
        let feature_title = feature_wu.title.to_lowercase();
        let has_feature_keyword = FEATURE_KEYWORDS.iter().any(|kw| {
            feature_title
                .strip_prefix(*kw)
                .is_some_and(|rest| rest.starts_with(' '))
        });
        if !has_feature_keyword {
            continue;
        }
        for &infra_wu in &work_units {
            if infra_wu.id == feature_wu.id {
                continue;
            }
            let infra_title = infra_wu.title.to_lowercase();
            let has_infra_keyword = INFRA_KEYWORDS.iter().any(|kw| infra_title.contains(kw));
            if !has_infra_keyword {
                continue;
            }
            let feature_prefix = prefix_of(&feature_wu.id);
            let infra_prefix = prefix_of(&infra_wu.id);
            if feature_prefix != infra_prefix {
                continue;
            }
            if relationship_contains(feature_wu, "dependsOn", &infra_wu.id)
                || relationship_contains(feature_wu, "blockedBy", &infra_wu.id)
            {
                continue;
            }
            suggestions.push(DependencySuggestion {
                from: feature_wu.id.clone(),
                to: infra_wu.id.clone(),
                r#type: "dependsOn".to_string(),
                reason: format!(
                    "infrastructure work (schema/migration) should complete before feature work: \"{}\" depends on \"{}\"",
                    feature_wu.title, infra_wu.title
                ),
                confidence: "high".to_string(),
            });
            specific_matches.insert(format!("{}->{}", feature_wu.id, infra_wu.id));
        }
    }

    // ── Rule 2 (TS): Sequential IDs in same prefix (MEDIUM fallback) ──
    // Group by prefix preserving first-seen order.
    let mut prefixes_order: Vec<String> = Vec::new();
    let mut by_prefix: std::collections::HashMap<String, Vec<&WorkUnit>> =
        std::collections::HashMap::new();
    for &wu in &work_units {
        let prefix = prefix_of(&wu.id);
        if !by_prefix.contains_key(&prefix) {
            prefixes_order.push(prefix.clone());
        }
        by_prefix.entry(prefix).or_default().push(wu);
    }

    for prefix in &prefixes_order {
        let Some(units) = by_prefix.get_mut(prefix) else {
            continue;
        };
        // Sort by numeric part of ID (stable so equal numbers keep order).
        units.sort_by_key(|wu| numeric_suffix(&wu.id));

        for i in 1..units.len() {
            let from = units[i];
            let to = units[i - 1];
            if relationship_contains(from, "dependsOn", &to.id)
                || relationship_contains(from, "blockedBy", &to.id)
            {
                continue;
            }
            if specific_matches.contains(&format!("{}->{}", from.id, to.id)) {
                continue;
            }
            suggestions.push(DependencySuggestion {
                from: from.id.clone(),
                to: to.id.clone(),
                r#type: "dependsOn".to_string(),
                reason: format!(
                    "sequential IDs in {prefix} prefix suggest {} depends on {}",
                    from.id, to.id
                ),
                confidence: "medium".to_string(),
            });
        }
    }

    // ── Rule 5 (TS): Remove circular dependencies ──
    filter_circular(suggestions)
}

/// Filter out one side of any reverse-pair, keeping the suggestion whose
/// `from < to` (lexicographic) — mirrors the TS `filter` predicate.
fn filter_circular(suggestions: Vec<DependencySuggestion>) -> Vec<DependencySuggestion> {
    // Compute a keep-mask first so we can move (not clone) the kept elements
    // out of the owned `suggestions` Vec.
    let keep: Vec<bool> = (0..suggestions.len())
        .map(|index| {
            let s = &suggestions[index];
            let has_reverse = suggestions
                .iter()
                .enumerate()
                .any(|(i, other)| i != index && other.from == s.to && other.to == s.from);
            !has_reverse || s.from < s.to
        })
        .collect();

    suggestions
        .into_iter()
        .zip(keep)
        .filter_map(|(s, keep)| keep.then_some(s))
        .collect()
}

/// Render the default numbered text summary. Mirrors the TS `output.log`
/// concatenation; the empty case prints the verbatim sentinel.
fn render_text(suggestions: &[DependencySuggestion]) -> String {
    if suggestions.is_empty() {
        return "No dependency suggestions found.\n\
Suggestions are based on sequential IDs, build/test pairs, and infrastructure patterns."
            .to_string();
    }

    let mut out = format!("\nFound {} dependency suggestion(s):\n\n", suggestions.len());
    for (index, s) in suggestions.iter().enumerate() {
        out.push_str(&format!("{}. {} → {} ({})\n", index + 1, s.from, s.to, s.r#type));
        out.push_str(&format!("   ● {}\n", s.reason));
        out.push_str(&format!("   Confidence: {}\n\n", s.confidence.to_uppercase()));
    }
    out.push_str("To apply a suggestion: fspec add-dependency <from-id> --depends-on=<to-id>");
    out
}

/// Return the prefix portion of an ID (`AUTH-001` → `AUTH`). Mirrors the TS
/// `wu.id.split('-')[0]`.
fn prefix_of(id: &str) -> String {
    id.split('-').next().unwrap_or(id).to_string()
}

/// Parse the numeric suffix of an ID (`AUTH-002` → `2`). Mirrors the TS
/// `parseInt(a.id.split('-')[1] || '0', 10)`.
fn numeric_suffix(id: &str) -> i64 {
    id.split('-')
        .nth(1)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// Whether `wu`'s `<field>` relationship array (top-level only, mirroring the
/// TS `wu.dependsOn?.includes(...)`) contains `id`.
fn relationship_contains(wu: &WorkUnit, field: &str, id: &str) -> bool {
    if let Some(Value::Array(arr)) = wu.extra.get(field) {
        return arr.iter().any(|v| v.as_str() == Some(id));
    }
    false
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn make_wu(id: &str, title: &str, extra: Value) -> WorkUnit {
        let mut v = json!({
            "id": id,
            "title": title,
            "type": "story",
            "status": "backlog",
            "createdAt": "x",
            "updatedAt": "x"
        });
        if let (Value::Object(base), Value::Object(ex)) = (&mut v, extra) {
            for (k, val) in ex {
                base.insert(k, val);
            }
        }
        serde_json::from_value(v).unwrap()
    }

    fn data_with(entries: &[(&str, &str, Value)]) -> WorkUnitsData {
        let mut data = WorkUnitsData::initial("x");
        for (id, title, extra) in entries {
            data.work_units
                .insert((*id).to_string(), make_wu(id, title, extra.clone()));
        }
        data
    }

    #[test]
    fn sequential_pair_is_medium() {
        let data = data_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({})),
        ]);
        let s = compute_suggestions(&data);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].from, "AUTH-002");
        assert_eq!(s[0].to, "AUTH-001");
        assert_eq!(s[0].confidence, "medium");
    }

    #[test]
    fn build_test_pair_is_high() {
        let data = data_with(&[
            ("BUILD-001", "Build authentication", json!({})),
            ("TEST-001", "Test authentication", json!({})),
        ]);
        let s = compute_suggestions(&data);
        let found = s
            .iter()
            .find(|x| x.from == "TEST-001" && x.to == "BUILD-001")
            .expect("test->build suggestion");
        assert_eq!(found.confidence, "high");
    }

    #[test]
    fn existing_dependson_excludes_sequential() {
        let data = data_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({ "dependsOn": ["AUTH-001"] })),
        ]);
        assert!(compute_suggestions(&data).is_empty());
    }

    #[test]
    fn specific_overrides_sequential() {
        let data = data_with(&[
            ("BUILD-001", "Build authentication", json!({})),
            ("BUILD-002", "Test authentication", json!({})),
        ]);
        let s = compute_suggestions(&data);
        let matching: Vec<&DependencySuggestion> = s
            .iter()
            .filter(|x| x.from == "BUILD-002" && x.to == "BUILD-001")
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].confidence, "high");
    }

    #[test]
    fn empty_text_render() {
        assert!(render_text(&[]).starts_with("No dependency suggestions found."));
    }
}
