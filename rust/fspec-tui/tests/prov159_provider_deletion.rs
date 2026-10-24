//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//! Primary feature: spec/features/prov159-provider-deletion.feature

#[path = "../../providers/tests/prov159_support/mod.rs"]
mod support;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_sessions::SessionManager;
use serial_test::serial;
use support::{Fixture, TestResult};

#[test]
#[serial]
fn scenario_a3_provider_deletion_preserves_shared_document_fields() -> TestResult {
    // @step Given the Codex provider uses OAuth account A from the keychain with a file fallback
    // @step When the user deletes the Codex provider and credentials are detected again after restarting
    // @step Then neither the keychain nor the file fallback restores the deleted OAuth account
    // @step And an independent OpenAI API key outside the deleted provider remains unchanged
    // @step And unrelated fields in the shared Codex credential document remain unchanged
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    codelet_common::set_data_directory(fixture.directory.path().to_owned())?;
    #[cfg(target_os = "macos")]
    {
        use keyring::credential::CredentialApi;
        fixture
            .keychain()
            .set_password(&serde_json::to_string(&support::auth("A"))?)?;
    }
    let manager = SessionManager::new();
    manager.delete_provider_credentials("codex")?;
    manager.delete_provider_credentials("codex")?;
    let auth = read_codex_auth()?.ok_or("provider deletion destroyed shared credentials")?;
    assert!(auth.tokens.is_none());
    assert_eq!(auth.openai_api_key.as_deref(), Some("sk-independent"));
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    assert_eq!(document["unrelated"]["keep"], true);
    Ok(())
}
