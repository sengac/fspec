// Feature: spec/features/provider-credential-deletion.feature
//
//! PROV-133 — pressing 'd' in provider settings must actually remove the
//! credential across EVERY source the availability projection reads, so the
//! next `list_provider_credentials` reports the provider as unconfigured.
//!
//! These are RED-phase integration tests written before the fix. The current
//! `SessionManager::delete_provider_credentials` only clears the
//! `credentials.json` entry (`credentials::delete_credential`) — it does NOT
//! unset the process env var and does NOT remove the OAuth auth file. But
//! `list_provider_credentials`/`ProviderCredentials::detect()` derives
//! `configured` from the ENV VAR + AUTH FILE (never credentials.json), so today
//! the delete is a silent no-op for OAuth providers and leaves the env-var
//! source intact for API-key providers. These tests therefore FAIL until the
//! delete becomes authoritative (Option A).
//!
//! Integration-first, no mocks: a fresh redirected fspec home + data dir per
//! test, a real `credentials.json`, real `claude_auth.json` / codex `auth.json`
//! files, and real `std::env` vars. "Confirm delete" invokes the SAME backend
//! function the dispatch wires to — `SessionManager::delete_provider_credentials`
//! (sessions/handle_impl.rs:1383) — via the `SessionManagerHandle` trait.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::ProviderCredentials;
use codelet_sessions::SessionManager;

/// Serializes every test in this binary. These tests mutate process-global
/// state — the API-key env vars, `FSPEC_HOME`, `CODEX_HOME`, and the global
/// data directory (`codelet_common::set_data_directory`). A parallel test would
/// clobber another's seeded credential state, so each test acquires this lock
/// for its whole body. Mirrors the `ENV_GUARD`/`DATA_DIR_GUARD` pattern used by
/// prov128/prov129/rpc073. Report: env-serialization approach = a file-scoped
/// `static ENV_GUARD: Mutex<()>` acquired at the top of each `#[test]`.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Every credential-source env var these tests may touch. Cleared at the start
/// (so no ambient/user credential leaks in) and removed again at teardown (so
/// this binary never poisons a sibling test suite).
const ALL_CRED_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// RAII guard: captures the prior values of the env vars + homes we mutate and
/// restores them (or removes them) on drop, even if the test panics. This is
/// the "restore/remove symmetrically at test end" requirement.
struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    /// Capture the current values of everything we may mutate, then clear all
    /// credential env vars so only what the test seeds decides configured-ness.
    fn capture_and_clear() -> Self {
        let mut names: Vec<&'static str> = ALL_CRED_VARS.to_vec();
        names.push("FSPEC_HOME");
        names.push("CODEX_HOME");
        let saved = names
            .into_iter()
            .map(|n| (n, std::env::var(n).ok()))
            .collect();
        for var in ALL_CRED_VARS {
            std::env::remove_var(var);
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Point `FSPEC_HOME`, `CODEX_HOME`, and the global data directory at a fresh
/// temp home so auth files + credentials.json land in the temp dir, never the
/// real `~/`. Returns the `TempDir` guard — keep it alive for the whole test.
///
/// `FSPEC_HOME` is used verbatim as the credentials dir by
/// `oauth::fspec_home()` (claude_auth.json lives directly under it). The global
/// data dir is where `credentials::delete_credential` reads/writes
/// `<data_dir>/credentials/credentials.json`. We point the data dir at
/// `FSPEC_HOME` too so both live under one temp home.
fn redirect_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("create temp fspec home");
    std::env::set_var("FSPEC_HOME", home.path());
    std::env::set_var("CODEX_HOME", home.path());
    codelet_common::set_data_directory(home.path().to_path_buf())
        .expect("set global data directory");
    home
}

/// Path to `credentials.json` under the given data dir.
fn credentials_json_path(home: &Path) -> std::path::PathBuf {
    home.join("credentials").join("credentials.json")
}

/// Seed a real `credentials.json` with a single `openai` api-key entry.
fn seed_openai_credentials_json(home: &Path) {
    let dir = home.join("credentials");
    fs::create_dir_all(&dir).expect("create credentials dir");
    fs::write(
        dir.join("credentials.json"),
        r#"{"version":1,"providers":{"openai":{"apiKey":"sk-openai-prov133","lastUpdated":"2026-07-04T00:00:00Z"}}}"#,
    )
    .expect("write credentials.json");
}

/// Seed a real `claude_auth.json` (Anthropic OAuth tokens) under `FSPEC_HOME`.
fn seed_claude_auth(home: &Path) {
    fs::write(
        home.join("claude_auth.json"),
        r#"{"access_token":"claude-access-prov133","refresh_token":"claude-refresh-prov133","expires":9999999999999}"#,
    )
    .expect("write claude_auth.json");
}

/// Seed a real codex `auth.json` (Codex OAuth tokens) under `CODEX_HOME`.
/// detect()'s `has_codex_auth` requires non-empty refresh_token + account_id.
fn seed_codex_auth(home: &Path) {
    fs::write(
        home.join("auth.json"),
        r#"{"tokens":{"id_token":"id-prov133","access_token":"access-prov133","refresh_token":"refresh-prov133","account_id":"acct-prov133"}}"#,
    )
    .expect("write codex auth.json");
}

/// Read the `configured` flag for a provider straight from the real projection
/// the TUI uses (`SessionManagerHandle::list_provider_credentials`).
fn configured(handle: &dyn SessionManagerHandle, provider_id: &str) -> bool {
    handle
        .list_provider_credentials()
        .into_iter()
        .find(|p| p.provider_id == provider_id)
        .map(|p| p.configured)
        .unwrap_or_else(|| panic!("provider {provider_id} must appear in the credential list"))
}

// =============================================================================
// Scenario: Deleting an API-key provider unsets its env var and clears
// credentials.json
// =============================================================================
#[test]
fn deleting_api_key_provider_unsets_env_and_clears_credentials_json() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::capture_and_clear();
    let home = redirect_home();
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = &manager;

    // @step Given OpenAI is configured via the OPENAI_API_KEY environment variable
    std::env::set_var("OPENAI_API_KEY", "sk-openai-prov133");
    assert!(
        ProviderCredentials::detect().has_openai(),
        "precondition: OpenAI must be detected as configured via env var"
    );

    // @step And OpenAI has an entry in credentials.json
    seed_openai_credentials_json(home.path());
    assert!(
        credentials_json_path(home.path()).exists(),
        "precondition: credentials.json seeded with an openai entry"
    );

    // @step When I confirm delete for OpenAI
    handle
        .delete_provider_credentials("openai")
        .expect("delete_provider_credentials should succeed");

    // @step Then the OPENAI_API_KEY environment variable is unset
    assert!(
        std::env::var("OPENAI_API_KEY").is_err(),
        "PROV-133: after delete, OPENAI_API_KEY must be unset (env source cleared)"
    );

    // @step And OpenAI has no entry in credentials.json
    let raw = fs::read_to_string(credentials_json_path(home.path())).unwrap_or_default();
    assert!(
        !raw.contains("openai"),
        "PROV-133: credentials.json must no longer contain an openai entry; got: {raw}"
    );

    // @step And the provider list reports OpenAI as not configured
    assert!(
        !configured(handle, "openai"),
        "PROV-133: OpenAI must report configured=false after delete"
    );
}

// =============================================================================
// Scenario: Deleting an OAuth provider removes its auth file
// =============================================================================
#[test]
fn deleting_oauth_provider_removes_its_auth_file() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::capture_and_clear();
    let home = redirect_home();
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = &manager;

    // @step Given Anthropic is configured via OAuth tokens in claude_auth.json
    seed_claude_auth(home.path());
    assert!(
        home.path().join("claude_auth.json").exists(),
        "precondition: claude_auth.json seeded"
    );
    assert!(
        ProviderCredentials::detect().has_claude(),
        "precondition: Anthropic must be detected as configured via claude_auth.json"
    );

    // @step When I confirm delete for Anthropic
    handle
        .delete_provider_credentials("anthropic")
        .expect("delete_provider_credentials should succeed");

    // @step Then the claude_auth.json auth file is removed
    assert!(
        !home.path().join("claude_auth.json").exists(),
        "PROV-133: after delete, claude_auth.json must be removed (auth-file source cleared)"
    );

    // @step And the provider list reports Anthropic as not configured
    assert!(
        !configured(handle, "anthropic"),
        "PROV-133: Anthropic must report configured=false after delete"
    );
}

// =============================================================================
// Scenario: Deleting one provider leaves other configured providers untouched
// =============================================================================
#[test]
fn deleting_one_provider_leaves_others_untouched() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::capture_and_clear();
    let home = redirect_home();
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = &manager;

    // @step Given Anthropic is configured via OAuth tokens in claude_auth.json
    seed_claude_auth(home.path());
    assert!(
        ProviderCredentials::detect().has_claude(),
        "precondition: Anthropic configured via claude_auth.json"
    );

    // @step And Codex is configured via OAuth tokens in its own auth.json
    seed_codex_auth(home.path());
    assert!(
        ProviderCredentials::detect().has_codex(),
        "precondition: Codex configured via codex auth.json"
    );

    // @step When I confirm delete for Anthropic
    handle
        .delete_provider_credentials("anthropic")
        .expect("delete_provider_credentials should succeed");

    // @step Then the provider list reports Anthropic as not configured
    assert!(
        !configured(handle, "anthropic"),
        "PROV-133: Anthropic must report configured=false after delete"
    );

    // @step And the provider list still reports Codex as configured
    assert!(
        configured(handle, "codex"),
        "PROV-133: Codex must remain configured=true — delete affects only the target"
    );
    assert!(
        home.path().join("auth.json").exists(),
        "PROV-133: codex auth.json must be untouched by an anthropic delete"
    );
}

// =============================================================================
// Scenario: Deleting a provider with no stored credential is a safe no-op
// =============================================================================
#[test]
fn deleting_provider_with_no_credential_is_safe_noop() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::capture_and_clear();
    let home = redirect_home();
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = &manager;

    // @step Given OpenAI has no environment variable, credentials.json entry, or auth file
    assert!(
        std::env::var("OPENAI_API_KEY").is_err(),
        "precondition: OPENAI_API_KEY is unset"
    );
    assert!(
        !credentials_json_path(home.path()).exists(),
        "precondition: no credentials.json exists"
    );
    assert!(
        !ProviderCredentials::detect().has_openai(),
        "precondition: OpenAI is not configured"
    );

    // @step When I confirm delete for OpenAI
    let result = handle.delete_provider_credentials("openai");

    // @step Then the delete succeeds without error
    assert!(
        result.is_ok(),
        "PROV-133: deleting an absent credential must be a safe no-op, got: {result:?}"
    );

    // @step And the provider list reports OpenAI as not configured
    assert!(
        !configured(handle, "openai"),
        "PROV-133: OpenAI must report configured=false"
    );
}
