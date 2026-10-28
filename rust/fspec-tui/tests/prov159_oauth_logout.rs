//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//! Primary feature: spec/features/prov159-oauth-logout.feature

#[path = "../../providers/tests/prov159_support/mod.rs"]
pub mod prov159_support;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_rpc::SharedFspecService;
#[cfg(target_os = "macos")]
use keyring::credential::CredentialApi;
use prov159_support::{Fixture, TestResult};
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn scenario_a1_local_logout_clears_every_source_and_preserves_api_key() -> TestResult {
    // @step Given Codex account A exists in the preferred keychain and file fallback alongside an independent OpenAI API key
    let fixture = Fixture::new()?;
    fixture.seed("A")?;
    #[cfg(target_os = "macos")]
    fixture
        .keychain()?
        .set_password(&serde_json::to_string(&prov159_support::auth("A"))?)?;
    let project = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    std::fs::write(
        project.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let watcher = Arc::new(WorkUnitsWatcher::new(project.path())?);
    let service = Arc::new(SharedFspecService::new(watcher));
    let backend = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service.clone());
    assert!(backend.oauth_get_tokens("codex".into()).await?);
    // @step When the user logs out of Codex OAuth and credentials are read again after restarting
    backend.oauth_clear_tokens("codex".into()).await?;
    // @step And repeating local logout succeeds without restoring OAuth credentials
    backend.oauth_clear_tokens("codex".into()).await?;
    drop(backend);
    let restarted = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service);
    // @step Then neither credential source restores OAuth account A
    assert!(
        !restarted.oauth_get_tokens("codex".into()).await?,
        "the preferred keychain resurrected logged-out OAuth tokens"
    );
    let stored = read_codex_auth()?.ok_or("independent API key must remain")?;
    assert!(stored.tokens.is_none());
    // @step And the independent OpenAI API key remains available
    assert_eq!(stored.openai_api_key.as_deref(), Some("sk-independent"));
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.directory.path().join("auth.json"))?)?;
    assert_eq!(document["unrelated"]["keep"], true);
    Ok(())
}
