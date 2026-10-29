/// Scenario: New wire types live in rust/rpc-types/src/lib.rs
#[test]
fn wire_types_live_in_rpc_types_lib_rs() {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let source = read_at("rust/rpc-types/src/lib.rs");
    // @step Then it declares public types ProviderCredentialInfo, ProviderCredentialInput, and TestConnectionResult
    assert!(source.contains("pub struct ProviderCredentialInfo"));
    assert!(source.contains("pub struct ProviderCredentialInput"));
    assert!(source.contains("pub struct TestConnectionResult"));
    // @step And each type has Serialize + Deserialize derives
    let cfg_attr_count = source
        .matches("cfg_attr(feature = \"napi\", napi_derive::napi(object))")
        .count();
    assert!(cfg_attr_count >= 3);
    assert!(source.contains("Serialize, Deserialize"));
    // @step And ProviderCredentialInfo is gated by #[cfg_attr(feature = "napi", napi_derive::napi(object))]
    let idx = source
        .find("pub struct ProviderCredentialInfo")
        .expect("ProviderCredentialInfo present");
    let tail_window = &source[..idx][source[..idx].len().saturating_sub(500)..];
    assert!(tail_window.contains("cfg_attr(feature = \"napi\", napi_derive::napi(object))"));
}

/// Scenario: The ProviderSettingsView module exists at the expected path
#[test]
fn provider_settings_view_module_exists() {
    // @step Given the workspace root
    // @step When rust/fspec-tui/src/views/provider_settings/mod.rs is read
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs");
    // @step Then the file exists
    // (asserted by read_at succeeding)
    // @step And it declares a pub struct ProviderSettingsView
    assert!(source.contains("pub struct ProviderSettingsView"));
    // PROV-113: the mode enums were extracted into a sibling `mode.rs` so the
    // new OAuth-login variants fit without pushing mod.rs past the 300-LoC
    // ceiling; mod.rs re-exports them. Assert the re-export here and the
    // declarations against mode.rs below.
    assert!(source.contains("pub use mode::{DetailSub, ProviderSettingsMode}"));
    let mode_source = read_at("rust/fspec-tui/src/views/provider_settings/mode.rs");
    // @step And it declares a pub enum ProviderSettingsMode with variants List and Detail
    assert!(mode_source.contains("pub enum ProviderSettingsMode"));
    assert!(mode_source.contains("List"));
    assert!(mode_source.contains("Detail"));
    // @step And it declares a pub enum DetailSub with variants Summary, EditApiKey, OAuthNotice
    assert!(mode_source.contains("pub enum DetailSub"));
    assert!(mode_source.contains("Summary"));
    assert!(mode_source.contains("EditApiKey"));
    assert!(mode_source.contains("OAuthNotice"));

    // And rust/fspec-tui/src/views/mod.rs declares pub mod provider_settings
    let views_mod = read_at("rust/fspec-tui/src/views/mod.rs");
    assert!(views_mod.contains("pub mod provider_settings"));
}
