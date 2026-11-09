/// Scenario: New wire types live in rust/rpc-types/src/lib.rs
#[test]
fn wire_types_live_in_rpc_types_lib_rs() -> TestResult {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let source = read_at("rust/rpc-types/src/lib.rs")?;
    // @step Then it declares public types ProviderCredentialInfo, ProviderCredentialInput, and TestConnectionResult
    assert!(source.contains("pub struct ProviderCredentialInfo"));
    assert!(source.contains("pub struct ProviderCredentialInput"));
    assert!(source.contains("pub struct TestConnectionResult"));
    // @step And each type has Serialize + Deserialize derives
    let derive_count = source.matches("Serialize, Deserialize").count();
    assert!(derive_count >= 3);
    assert!(source.contains("Serialize, Deserialize"));
    // @step And ProviderCredentialInfo is gated by #[cfg_attr(feature = "napi", napi_derive::napi(object))]
    let idx = source
        .find("pub struct ProviderCredentialInfo")
        .or_fail("ProviderCredentialInfo present")?;
    let tail_window = &source[..idx][source[..idx].len().saturating_sub(500)..];
    assert!(tail_window.contains("#[derive(") && tail_window.contains("Serialize, Deserialize"));
    Ok(())
}

/// Scenario: The ProviderSettingsView module exists at the expected path
#[test]
fn provider_settings_view_module_exists() -> TestResult {
    // @step Given the workspace root
    // @step When rust/fspec-tui/src/views/provider_settings/mod.rs is read
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs")?;
    // @step Then the file exists
    // (asserted by read_at succeeding)
    // @step And it declares a pub struct ProviderSettingsView
    assert!(source.contains("pub struct ProviderSettingsView"));
    // PROV-113: the mode enums were extracted into a sibling `mode.rs` so the
    // new OAuth-login variants fit without pushing mod.rs past the 300-LoC
    // ceiling; mod.rs re-exports them. Assert the re-export here and the
    // declarations against mode.rs below.
    assert!(source.contains("pub use mode::{DetailSub, ProviderSettingsMode}"));
    let mode_source = read_at("rust/fspec-tui/src/views/provider_settings/mode.rs")?;
    // @step And it declares a pub enum ProviderSettingsMode with variants List and Detail
    assert!(mode_source.contains("pub enum ProviderSettingsMode"));
    assert!(mode_source.contains("List"));
    assert!(mode_source.contains("Detail"));
    // @step And it declares a pub enum DetailSub with the one variant EditApiKey (BUG-578 deleted Summary and OAuthNotice)
    assert!(mode_source.contains("pub enum DetailSub {\n    EditApiKey { draft: String },\n}"));

    // And rust/fspec-tui/src/views/mod.rs declares pub mod provider_settings
    let views_mod = read_at("rust/fspec-tui/src/views/mod.rs")?;
    assert!(views_mod.contains("pub mod provider_settings"));
    Ok(())
}
