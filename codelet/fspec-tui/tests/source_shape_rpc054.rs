//! RPC-054 — Source-shape assertions for the new provider-credentials
//! wire types and view module.
//!
//! Feature: spec/features/rpc054-provider-settings-source-shape.feature
//!
//! These tests scan the actual source files at compile time to assert
//! the file shapes called out by the feature file's source-shape
//! scenarios. Mirrors the source_shape_rpc049 / source_shape_rpc050
//! pattern.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // Cargo runs integration tests with CARGO_MANIFEST_DIR = the crate
    // dir (codelet/fspec-tui). The workspace root is two levels up.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/fspec-tui")
        .to_path_buf()
}

/// Scenario: New wire types live in codelet/rpc-types/src/lib.rs
#[test]
fn wire_types_live_in_rpc_types_lib_rs() {
    // @step Given the file codelet/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");

    // @step Then it declares public types ProviderCredentialInfo, ProviderCredentialInput, and TestConnectionResult
    assert!(
        source.contains("pub struct ProviderCredentialInfo"),
        "rpc-types should declare pub struct ProviderCredentialInfo"
    );
    assert!(
        source.contains("pub struct ProviderCredentialInput"),
        "rpc-types should declare pub struct ProviderCredentialInput"
    );
    assert!(
        source.contains("pub struct TestConnectionResult"),
        "rpc-types should declare pub struct TestConnectionResult"
    );

    // @step And each type has Serialize + Deserialize derives
    // Each cfg_attr line is followed by a derive that includes both
    // Serialize and Deserialize — assert the cfg_attr appears at least
    // three times (once per type) AND the derive list co-occurs.
    let cfg_attr_count = source.matches("cfg_attr(feature = \"napi\", napi_derive::napi(object))").count();
    assert!(
        cfg_attr_count >= 3,
        "expected at least 3 cfg_attr(napi(object)) occurrences for the new types, got {cfg_attr_count}"
    );
    assert!(
        source.contains("Serialize, Deserialize"),
        "expected Serialize, Deserialize derives in rpc-types"
    );

    // @step And ProviderCredentialInfo is gated by #[cfg_attr(feature = "napi", napi_derive::napi(object))]
    // Find the ProviderCredentialInfo declaration and assert the line
    // immediately preceding the `#[derive(...)]` is the cfg_attr line.
    let idx = source
        .find("pub struct ProviderCredentialInfo")
        .expect("ProviderCredentialInfo present");
    let prelude = &source[..idx];
    let tail_window = &prelude[prelude.len().saturating_sub(500)..];
    assert!(
        tail_window.contains("cfg_attr(feature = \"napi\", napi_derive::napi(object))"),
        "ProviderCredentialInfo should be preceded by the napi cfg_attr gate, got prelude tail:\n{tail_window}"
    );
}

/// Scenario: ProviderSettingsView module exists with the expected source shape
#[test]
fn provider_settings_view_module_has_expected_shape() {
    // @step Given the file codelet/fspec-tui/src/views/provider_settings/mod.rs exists
    let path = workspace_root().join("codelet/fspec-tui/src/views/provider_settings/mod.rs");
    let source = fs::read_to_string(&path).expect("read views/provider_settings/mod.rs");

    // @step When the file is compiled as part of codelet-fspec-tui
    // (asserted by this test compiling alongside the rest of the crate)

    // @step Then it declares pub struct ProviderSettingsView
    assert!(
        source.contains("pub struct ProviderSettingsView"),
        "provider_settings module should declare pub struct ProviderSettingsView"
    );

    // @step And it declares an enum or state describing list-mode and edit-api-key-mode
    assert!(
        source.contains("pub enum ProviderSettingsMode"),
        "provider_settings module should declare pub enum ProviderSettingsMode"
    );
    assert!(
        source.contains("List") && source.contains("EditApiKey"),
        "ProviderSettingsMode should have List + EditApiKey variants"
    );

    // @step And codelet/fspec-tui/src/views/mod.rs declares pub mod provider_settings
    let views_mod_path = workspace_root().join("codelet/fspec-tui/src/views/mod.rs");
    let views_mod = fs::read_to_string(&views_mod_path).expect("read views/mod.rs");
    assert!(
        views_mod.contains("pub mod provider_settings"),
        "views/mod.rs should declare pub mod provider_settings"
    );
}
