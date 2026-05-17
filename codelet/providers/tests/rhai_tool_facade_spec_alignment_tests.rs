#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]
//! Feature: spec/features/reconcile-rhaitoolfacadeadapter-spec-with-implementation.feature
//!
//! Integration tests for PROV-068: reconcile `RhaiToolFacadeAdapter`
//! spec (PROV-066 rules + architecture notes + PROV-061 architecture
//! note + feature file doc string) with the actual getters-only
//! implementation.
//!
//! These tests assert two things:
//!
//! 1. The runtime contract of `RhaiToolFacadeAdapter` — the `name()`,
//!    `parameters_schema()`, and `maps_to()` getters return the
//!    values supplied via `RhaiToolDef`. Without these, the
//!    "getters-only" design claim has no regression protection.
//!
//! 2. The PROV-066 feature file background doc string no longer
//!    references `rig::Tool`, confirming Option A reconciliation was
//!    actually applied to the living documentation.
//!
//! The PROV-066 rules/arch notes and PROV-061 arch notes live in
//! `spec/work-units.json` and are edited via fspec commands (not
//! directly asserted here — fspec's own tests cover JSON store
//! invariants). The feature-file check provides a concrete,
//! testable anchor in Rust.

use std::sync::Arc;

#[path = "custom_tool_facades_test_helpers.rs"]
mod helpers;

use helpers::{facade_config_with_script, make_loader, NO_TOOL_FNS_SCRIPT};

use codelet_providers::custom::tool_facade::{RhaiToolDef, RhaiToolFacadeAdapter};
use codelet_providers::custom::ToolStyle;

/// Build a `RhaiToolDef` suitable for exercising every getter in one
/// test, so individual scenarios stay focused.
fn sample_def() -> RhaiToolDef {
    RhaiToolDef {
        name: "my_read".to_string(),
        description: "read a file".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        }),
        maps_to: "file:read".to_string(),
    }
}

fn build_adapter(def: RhaiToolDef) -> (tempfile::TempDir, RhaiToolFacadeAdapter) {
    let (tmp, cfg) = facade_config_with_script("my-llm", NO_TOOL_FNS_SCRIPT, ToolStyle::Claude);
    let loader = make_loader();
    let adapter =
        RhaiToolFacadeAdapter::new(Arc::new(def), Arc::new(cfg), Arc::clone(&loader))
            .expect("build adapter");
    // Return TempDir so the caller controls its lifetime; the adapter
    // stores `Arc` handles and does not re-read the script file, but
    // keeping TempDir alive avoids surprising teardown ordering.
    (tmp, adapter)
}

// =========================================================================
// Scenario: Adapter name getter returns the Rhai-supplied tool name
// =========================================================================
#[test]
fn adapter_name_getter_returns_rhai_supplied_tool_name() {
    // @step Given a RhaiToolDef with name "my_read", description "read a file", and a parameters schema, and maps_to "file:read"
    let def = sample_def();

    // @step When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    let (_tmp, adapter) = build_adapter(def);

    // @step Then RhaiToolFacadeAdapter.name() returns "my_read"
    assert_eq!(adapter.name(), "my_read");
}

// =========================================================================
// Scenario: Adapter parameters_schema getter returns the Rhai-supplied schema
// =========================================================================
#[test]
fn adapter_parameters_schema_getter_returns_rhai_supplied_schema() {
    // @step Given a RhaiToolDef with parameters schema {"type":"object","properties":{"path":{"type":"string"}}}
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" }
        }
    });
    let def = RhaiToolDef {
        name: "my_read".to_string(),
        description: "read a file".to_string(),
        parameters: schema.clone(),
        maps_to: "file:read".to_string(),
    };

    // @step When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    let (_tmp, adapter) = build_adapter(def);

    // @step Then RhaiToolFacadeAdapter.parameters_schema() returns a &serde_json::Value equal to that schema
    assert_eq!(adapter.parameters_schema(), &schema);
}

// =========================================================================
// Scenario: Adapter maps_to getter exposes the routing identifier
// =========================================================================
#[test]
fn adapter_maps_to_getter_exposes_routing_identifier() {
    // @step Given a RhaiToolDef with maps_to "file:read"
    let def = sample_def();

    // @step When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    let (_tmp, adapter) = build_adapter(def);

    // @step Then RhaiToolFacadeAdapter.maps_to() returns the string "file:read"
    assert_eq!(adapter.maps_to(), "file:read");
}

// =========================================================================
// Scenario: PROV-066 Rule 0 describes getters-only adapter design
// =========================================================================
#[test]
fn prov_066_rule_0_describes_getters_only_adapter_design() {
    // @step Given the PROV-066 work unit rules have been reconciled with the implementation
    let json = read_work_units_json();
    let active_rules = collect_active_item_texts(&json, "PROV-066", "rules");

    // @step When I read PROV-066 rule with stable id 0
    // After reconciliation, the rule claiming "implementing rig::Tool"
    // is soft-deleted. We check the full active rules list to confirm
    // (a) a getters-only description exists and (b) no active rule
    // claims rig::Tool implementation.

    // @step Then the rule text states that RhaiToolFacadeAdapter is a getters-only adapter and does not claim it implements rig::Tool
    let any_getters_only = active_rules
        .iter()
        .any(|r| r.contains("RhaiToolFacadeAdapter") && r.contains("getters-only"));
    assert!(
        any_getters_only,
        "PROV-066 rules should contain a getters-only RhaiToolFacadeAdapter rule, got: {active_rules:?}"
    );
    for r in &active_rules {
        assert!(
            !is_positive_rig_tool_claim(r),
            "no active PROV-066 rule should positively claim rig::Tool implementation, got: {r}"
        );
    }
}

// =========================================================================
// Scenario: PROV-066 architecture note 0 describes getters-only adapter design
// =========================================================================
#[test]
fn prov_066_architecture_note_0_describes_getters_only_adapter_design() {
    // @step Given the PROV-066 architecture notes have been reconciled with the implementation
    let json = read_work_units_json();
    let active_notes = collect_active_item_texts(&json, "PROV-066", "architectureNotes");

    // @step When I read PROV-066 architecture note with stable id 0
    // The original note was soft-deleted; a replacement active note
    // is asserted here.

    // @step Then the note text describes RhaiToolFacadeAdapter as a getters-only adapter and does not claim it implements rig::Tool
    let any_getters_only = active_notes
        .iter()
        .any(|n| n.contains("RhaiToolFacadeAdapter") && n.contains("getters-only"));
    assert!(
        any_getters_only,
        "PROV-066 architecture notes should contain a getters-only RhaiToolFacadeAdapter note, got: {active_notes:?}"
    );
    for n in &active_notes {
        assert!(
            !is_positive_rig_tool_claim(n),
            "no active PROV-066 architecture note should positively claim rig::Tool implementation, got: {n}"
        );
    }
}

// =========================================================================
// Scenario: PROV-061 architecture note 1 describes getters-only adapter design
// =========================================================================
#[test]
fn prov_061_architecture_note_1_describes_getters_only_adapter_design() {
    // @step Given the PROV-061 architecture notes have been reconciled with the implementation
    let json = read_work_units_json();
    let active_notes = collect_active_item_texts(&json, "PROV-061", "architectureNotes");

    // @step When I read PROV-061 architecture note with stable id 1
    // After reconciliation the original note at stable id 1 is
    // soft-deleted; a replacement active note describes the
    // getters-only design.

    // @step Then the note text describes RhaiToolFacadeAdapter as a getters-only adapter and does not claim it implements rig::Tool
    let any_getters_only = active_notes
        .iter()
        .any(|n| n.contains("RhaiToolFacadeAdapter") && n.contains("getters-only"));
    assert!(
        any_getters_only,
        "PROV-061 architecture notes should contain a getters-only RhaiToolFacadeAdapter note, got: {active_notes:?}"
    );
    for n in &active_notes {
        assert!(
            !is_positive_rig_tool_claim(n),
            "no active PROV-061 architecture note should positively claim rig::Tool implementation, got: {n}"
        );
    }
}

/// Return true when `text` positively asserts that RhaiToolFacadeAdapter
/// implements `rig::Tool`. Negated phrasing ("not a full rig::Tool impl")
/// is tolerated because those sentences are the reconciliation itself.
fn is_positive_rig_tool_claim(text: &str) -> bool {
    let lowered = text.to_lowercase();
    let phrases = [
        "implementing rig::tool",
        "implements rig::tool",
        "rig::tool implementation",
    ];
    for phrase in phrases {
        if let Some(idx) = lowered.find(phrase) {
            // Look at up to 40 characters BEFORE the phrase to see if
            // it is negated by "not", "not a", "not a full", etc.
            let start = idx.saturating_sub(40);
            let prefix = &lowered[start..idx];
            if prefix.contains("not ") || prefix.contains("isn't") || prefix.contains("is not") {
                continue;
            }
            return true;
        }
    }
    false
}

// =========================================================================
// Scenario: PROV-066 feature file no longer references rig::Tool semantics
// =========================================================================
#[test]
fn prov_066_feature_file_no_longer_references_rig_tool_semantics() {
    // @step Given the custom-provider-rhai-scriptable-tool-facades feature file has been reconciled
    let path = repo_root()
        .join("spec/features/custom-provider-rhai-scriptable-tool-facades.feature");
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read feature file {}: {e}", path.display()));

    // @step When I read the feature file background doc string and scenario step text
    // The file parses as a Gherkin document; we strip comment lines (# …)
    // so # BUSINESS RULES blocks listing historical context don't
    // trip the assertion.
    let non_comment_body: String = contents
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // @step Then no positive rig::Tool implementation claim remains in the background doc string or any scenario steps
    // Tolerate explicit negations (e.g. "not a full rig::Tool impl")
    // because the reconciliation itself may mention rig::Tool in a
    // negated form to document the design decision. What must NOT
    // remain is any positive claim of rig::Tool implementation.
    assert!(
        !contains_positive_rig_tool_claim(&non_comment_body),
        "PROV-066 feature file background/scenarios should not positively claim rig::Tool implementation. \n\
         Offending content:\n{non_comment_body}"
    );
}

/// Scan `haystack` for positive assertions of rig::Tool implementation
/// and return true if any are found. Mirrors the work-unit-JSON
/// assertion helper but operates on arbitrary multi-line strings.
fn contains_positive_rig_tool_claim(haystack: &str) -> bool {
    let lowered = haystack.to_lowercase();
    let phrases = [
        "implementing rig::tool",
        "implements rig::tool",
        "rig::tool implementation",
        "rig::tool name",
    ];
    for phrase in phrases {
        let mut search_start = 0;
        while let Some(idx_rel) = lowered[search_start..].find(phrase) {
            let idx = search_start + idx_rel;
            let prefix_start = idx.saturating_sub(40);
            let prefix = &lowered[prefix_start..idx];
            let negated = prefix.contains("not ")
                || prefix.contains("isn't")
                || prefix.contains("is not")
                || prefix.contains("no ");
            if !negated {
                return true;
            }
            search_start = idx + phrase.len();
        }
    }
    false
}

// -------------------------------------------------------------------------
// Helpers: fspec work-units.json access
// -------------------------------------------------------------------------

fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR → codelet/providers; go up twice for workspace root.
    let providers_crate = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    providers_crate
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(&providers_crate)
        .to_path_buf()
}

fn read_work_units_json() -> serde_json::Value {
    let path = repo_root().join("spec/work-units.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("parse work-units.json: {e}"))
}

/// Locate a work unit object by ID inside work-units.json.
///
/// The fspec schema currently nests units under `workUnits` keyed by
/// ID, but that has changed historically — this helper walks both
/// common shapes (object map or array of `{id, ...}`) so the test
/// stays robust.
fn find_work_unit<'a>(
    json: &'a serde_json::Value,
    id: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(map) = json.get("workUnits").and_then(|v| v.as_object()) {
        if let Some(unit) = map.get(id) {
            return Some(unit);
        }
    }
    if let Some(arr) = json.get("workUnits").and_then(|v| v.as_array()) {
        for unit in arr {
            if unit.get("id").and_then(|v| v.as_str()) == Some(id) {
                return Some(unit);
            }
        }
    }
    // Fallback: scan the whole document for any object with matching id.
    if let Some(obj) = json.as_object() {
        for value in obj.values() {
            if let Some(arr) = value.as_array() {
                for unit in arr {
                    if unit.get("id").and_then(|v| v.as_str()) == Some(id) {
                        return Some(unit);
                    }
                }
            }
        }
    }
    None
}

/// Return the `text` field of the `rules[<idx>]` entry for the given
/// work unit, ignoring soft-deleted items. Stable IDs in the fspec
/// schema are stored as `id` on each rule entry.
fn lookup_rule_text(json: &serde_json::Value, id: &str, rule_id: u64) -> Option<String> {
    let unit = find_work_unit(json, id)?;
    lookup_item_text_by_stable_id(unit, "rules", rule_id)
}

/// Return the `text` field of the `architectureNotes[<idx>]` entry
/// for the given work unit.
fn lookup_architecture_note_text(
    json: &serde_json::Value,
    id: &str,
    note_id: u64,
) -> Option<String> {
    let unit = find_work_unit(json, id)?;
    lookup_item_text_by_stable_id(unit, "architectureNotes", note_id)
}

/// Stable-index lookup: items live as array of `{id, text, deleted?}`
/// objects, or as plain strings when the schema is flat. The public
/// `show-work-unit` output flattens strings, so we accept both shapes.
fn lookup_item_text_by_stable_id(
    unit: &serde_json::Value,
    field: &str,
    stable_id: u64,
) -> Option<String> {
    let arr = unit.get(field)?.as_array()?;
    for entry in arr {
        if let Some(obj) = entry.as_object() {
            let deleted = obj
                .get("deleted")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if deleted {
                continue;
            }
            let id_match = obj
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .map(|n| n == stable_id)
                .unwrap_or(false);
            if id_match {
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    return Some(text.to_string());
                }
            }
        }
    }
    // Fallback: flat string arrays, stable id == index.
    let idx = stable_id as usize;
    if let Some(entry) = arr.get(idx) {
        if let Some(s) = entry.as_str() {
            return Some(s.to_string());
        }
    }
    None
}

/// Collect the `text` fields of every non-deleted entry in the named
/// array field on a work unit. Used when assertions need to scan all
/// live rules/architecture notes rather than indexing by stable id.
fn collect_active_item_texts(
    json: &serde_json::Value,
    id: &str,
    field: &str,
) -> Vec<String> {
    let Some(unit) = find_work_unit(json, id) else {
        return Vec::new();
    };
    let Some(arr) = unit.get(field).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in arr {
        if let Some(obj) = entry.as_object() {
            let deleted = obj
                .get("deleted")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if deleted {
                continue;
            }
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                out.push(text.to_string());
            }
        } else if let Some(s) = entry.as_str() {
            out.push(s.to_string());
        }
    }
    out
}
