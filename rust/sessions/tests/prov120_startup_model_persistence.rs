//! PROV-120 — persisted startup model string read + legacy back-compat.
//!
//! Feature: spec/features/startup-model-initialization-first-available.feature
//!
//! Offline integration tests for the TS-parity persistence reader
//! (`loadPersistedModelString` port). Each test writes a throwaway
//! `fspec-config.json` (and, for the back-compat case, a legacy
//! `default-model.json`) into a temp directory and reads it back through the
//! path-injectable core, so the suite never touches the real `~/.fspec` and
//! never races on a process-global env var.
//!
//! TASK-3 RED phase: the reader is a stub returning `None`, so these
//! assertions are expected to FAIL until the green task implements the read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;

use codelet_sessions::last_used_model_persistence::load_persisted_model_string_from;
use tempfile::TempDir;

/// Scenario: Persisted model is restored from fspec-config.json tui.lastUsedModel
#[test]
fn restores_from_fspec_config_tui_last_used_model() {
    // @step Given the user fspec-config.json records tui.lastUsedModel
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("fspec-config.json"),
        r#"{"tui":{"lastUsedModel":"anthropic/claude-opus-4"}}"#,
    )
    .unwrap();
    // @step And that model matches a reachable credentialed section that still contains it
    // (section matching is the resolver's concern; this test covers the read step that
    //  feeds the resolver its persisted string.)

    // @step When startup model initialization runs
    let persisted = load_persisted_model_string_from(dir.path());

    // @step Then the persisted model string is read from fspec-config.json tui.lastUsedModel
    assert_eq!(persisted.as_deref(), Some("anthropic/claude-opus-4"));
    // @step And the default model resolves to that persisted model
    // (resolution is exercised by the resolver unit tests; here the read must surface
    //  the exact persisted string so the resolver can restore it.)
    assert!(persisted.is_some());
}

/// Scenario: Legacy default-model.json is read for continuity when fspec-config.json has no tui.lastUsedModel
#[test]
fn reads_legacy_default_model_json_when_config_has_none() {
    // @step Given the user fspec-config.json records no tui.lastUsedModel
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("fspec-config.json"), r#"{"theme":"dark"}"#).unwrap();
    // @step And a legacy default-model.json records a model that matches a reachable credentialed section
    fs::write(
        dir.path().join("default-model.json"),
        r#"{"model":"openai/gpt-4o"}"#,
    )
    .unwrap();

    // @step When startup model initialization runs
    let persisted = load_persisted_model_string_from(dir.path());

    // @step Then the persisted model is read once from the legacy default-model.json
    assert_eq!(persisted.as_deref(), Some("openai/gpt-4o"));
    // @step And the default model resolves to that legacy model
    assert!(persisted.is_some());
    // @step And subsequent model selection writes are written to fspec-config.json tui.lastUsedModel
    // (the write reconciliation is asserted in the green implementation task; this read-side
    //  test pins the back-compat READ that unblocks continuity.)
}
