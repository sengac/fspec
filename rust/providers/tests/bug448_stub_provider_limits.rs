//! BUG-448 — the test-only stub provider resolves the limits it declares.
//! BUG-548 — whatever logins the machine has: the credential homes hold
//! Claude and Codex logins, and the manager is still built for the stub.
//!
//! Feature: spec/features/test-stub-provider-declares-its-limits.feature
#![cfg(feature = "test-support")]

use codelet_providers::{ProviderManager, ProviderType};
use serde_json::json;

/// Point every credential home at `root` and give it a Claude and a Codex
/// login, the state of a developer machine with both providers credentialed.
/// This binary holds one test, so the process environment is its own.
fn credential_homes_with_claude_and_codex_logins(
    root: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let credentials = root.join("credentials");
    let codex = root.join("codex");
    std::fs::create_dir_all(&credentials)?;
    std::fs::create_dir_all(&codex)?;
    std::fs::write(
        credentials.join("claude_auth.json"),
        serde_json::to_vec(&json!({
            "access_token": "bug548-access",
            "refresh_token": "bug548-refresh",
            "expires": u64::MAX
        }))?,
    )?;
    std::fs::write(
        codex.join("auth.json"),
        serde_json::to_vec(&json!({"OPENAI_API_KEY": "bug548-codex"}))?,
    )?;
    std::env::set_var("FSPEC_HOME", &credentials);
    std::env::set_var("CODEX_HOME", &codex);
    std::env::set_var("FSPEC_USER_DIR", root);
    Ok(())
}

#[test]
fn the_stub_providers_declared_limits_are_resolved() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    // @step Given the credential homes hold Claude and Codex logins and the test-only stub provider is registered and selected
    credential_homes_with_claude_and_codex_logins(root.path())?;
    codelet_providers::stub_provider::register_stub_provider();
    // The stub needs no credentials: the manager is built for it explicitly,
    // so the logins in the credential homes are never counted.
    let mut manager = ProviderManager::for_testing(ProviderType::Custom("stub".into()), None, None);
    manager.set_model_direct("stub", "canned", None, None, None)?;
    // @step When its context window and output limit are resolved
    let limits = (manager.context_window(), manager.max_output_tokens());
    // @step Then they are the stub's declared 200000 and 4096 tokens, with no ambiguous-provider error
    assert_eq!(limits, (Some(200_000), Some(4_096)));
    Ok(())
}
