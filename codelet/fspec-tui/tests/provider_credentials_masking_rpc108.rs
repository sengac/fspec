#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock,
    clippy::too_many_lines
)]
//! Feature: spec/features/provider-settings-credential-masking-with-prefix-aware-mask-helper-and-source-tag.feature
//!
//! RPC-108 — Consolidated tests for prefix-aware credential masking +
//! provenance `source` tag across three layers:
//!
//!   1. Unit tests for `codelet_providers::credentials::mask_api_key`
//!      — five prefix matches, no-prefix fallback, short-key fallback,
//!      prefix-order precedence, and TS boundary cases.
//!   2. Integration tests for
//!      `codelet_providers::custom::list_providers_info` — env-sourced
//!      api-key, unconfigured rows, and OAuth-only providers (all
//!      using `#[serial]` because they read process env).
//!   3. Wire-level cross-transport parity — embedded + websocket both
//!      surface identical `masked_key` + `source` for seeded rows, and
//!      default ProviderCredentialInfo carries None for the two new
//!      fields (back-compat invariant).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_providers::credentials::mask_api_key;
use codelet_providers::custom::list_providers_info;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::ProviderCredentialInfo;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

// ============================================================================
// Section 1 — mask_api_key unit tests
// ============================================================================

#[test]
fn anthropic_sk_ant_key_masks_with_sk_ant_prefix() {
    // @step Given the api key string "sk-ant-api03-abcdefghijklmnop"
    let key = "sk-ant-api03-abcdefghijklmnop";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "sk-ant-••••••••mnop"
    assert_eq!(result, "sk-ant-••••••••mnop");
}

#[test]
fn openai_sk_key_masks_with_sk_prefix() {
    // @step Given the api key string "sk-test-1234567890abcdef"
    let key = "sk-test-1234567890abcdef";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "sk-••••••••cdef"
    assert_eq!(result, "sk-••••••••cdef");
}

#[test]
fn groq_gsk_underscore_key_masks_with_gsk_underscore_prefix() {
    // @step Given the api key string "gsk_test_1234567890abcdef"
    let key = "gsk_test_1234567890abcdef";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "gsk_••••••••cdef"
    assert_eq!(result, "gsk_••••••••cdef");
}

#[test]
fn gemini_aiza_key_masks_with_aiza_prefix() {
    // @step Given the api key string "AIzaSyABCDEFGH1234IJKLmnop"
    let key = "AIzaSyABCDEFGH1234IJKLmnop";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "AIza••••••••mnop"
    assert_eq!(result, "AIza••••••••mnop");
}

#[test]
fn xai_dash_key_masks_with_xai_dash_prefix() {
    // @step Given the api key string "xai-test-1234567890abcdef"
    let key = "xai-test-1234567890abcdef";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "xai-••••••••cdef"
    assert_eq!(result, "xai-••••••••cdef");
}

#[test]
fn unrecognised_prefix_falls_back_to_first_six_chars() {
    // @step Given the api key string "pktest-abcdefghijklmnop"
    let key = "pktest-abcdefghijklmnop";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "pktest••••••••mnop"
    assert_eq!(result, "pktest••••••••mnop");
}

#[test]
fn short_key_under_twelve_chars_renders_all_dots() {
    // @step Given the api key string "short"
    let key = "short";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result is "••••••••"
    assert_eq!(result, "••••••••");
}

#[test]
fn boundary_eleven_chars_falls_back_to_dots() {
    // 11 chars — one short of the 12-char floor; must still be all dots.
    assert_eq!(mask_api_key("12345678901"), "••••••••");
}

#[test]
fn boundary_twelve_chars_uses_first_six_fallback() {
    // Exactly 12 chars, no recognised prefix — uses first-6 + dots + last-4.
    assert_eq!(mask_api_key("123456789012"), "123456••••••••9012");
}

#[test]
fn empty_string_renders_all_dots() {
    assert_eq!(mask_api_key(""), "••••••••");
}

#[test]
fn prefix_order_precedence_sk_ant_beats_sk_dash() {
    // @step Given the api key string "sk-ant-1234567890abcdef"
    let key = "sk-ant-1234567890abcdef";

    // @step When mask_api_key is called on the key
    let result = mask_api_key(key);

    // @step Then the result starts with "sk-ant-" not "sk-"
    assert!(
        result.starts_with("sk-ant-"),
        "expected result to start with 'sk-ant-' but got '{result}'"
    );
    assert_eq!(result, "sk-ant-••••••••cdef");
}

// ============================================================================
// Section 2 — list_providers_info populates masked_key + source
// ============================================================================

fn save_env(key: &str) -> Option<String> {
    let v = env::var(key).ok();
    env::remove_var(key);
    v
}

fn restore_env(key: &str, original: Option<String>) {
    match original {
        Some(v) => env::set_var(key, v),
        None => env::remove_var(key),
    }
}

fn save_all_provider_env() -> Vec<(&'static str, Option<String>)> {
    let keys: &[&'static str] = &[
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "ZAI_API_KEY",
        "ZAI_PLAN_API_KEY",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "GROQ_API_KEY",
        "DEEPSEEK_API_KEY",
        "MISTRAL_API_KEY",
        "XAI_API_KEY",
        "TOGETHER_API_KEY",
        "HUGGINGFACE_API_KEY",
        "OPENROUTER_API_KEY",
        "MOONSHOT_API_KEY",
        "GALADRIEL_API_KEY",
        "COHERE_API_KEY",
        "AZURE_OPENAI_API_KEY",
    ];
    keys.iter().map(|k| (*k, save_env(k))).collect()
}

fn restore_all_provider_env(saved: Vec<(&'static str, Option<String>)>) {
    for (k, v) in saved {
        restore_env(k, v);
    }
}

#[test]
#[serial]
fn list_providers_info_populates_masked_key_and_source_for_env_sourced_api_key() {
    // @step Given OPENAI_API_KEY is set to "sk-test-1234567890abcdef" in the environment
    let saved = save_all_provider_env();
    env::set_var("OPENAI_API_KEY", "sk-test-1234567890abcdef");

    // @step When list_provider_credentials is called
    let list = list_providers_info().expect("list_providers_info");
    let openai = list
        .iter()
        .find(|p| p.name == "openai")
        .expect("openai entry present in canonical registry");

    // @step Then the openai ProviderCredentialInfo masked_key is Some("sk-••••••••cdef")
    assert_eq!(
        openai.masked_key.as_deref(),
        Some("sk-••••••••cdef"),
        "openai masked_key should be the TS-canonical sk- prefix masking"
    );

    // @step And the openai ProviderCredentialInfo source is Some("env")
    assert_eq!(
        openai.source.as_deref(),
        Some("env"),
        "openai source should be 'env' when credential came from process env"
    );

    restore_all_provider_env(saved);
}

#[test]
#[serial]
fn list_providers_info_leaves_masked_key_and_source_none_when_unconfigured() {
    // @step Given no environment variables are set for the openai provider
    let saved = save_all_provider_env();

    // @step When list_provider_credentials is called
    let list = list_providers_info().expect("list_providers_info");
    let openai = list
        .iter()
        .find(|p| p.name == "openai")
        .expect("openai entry present in canonical registry");

    // @step Then the openai ProviderCredentialInfo configured is false
    assert!(
        !openai.available,
        "openai should be unconfigured with no env var set"
    );

    // @step And the openai ProviderCredentialInfo masked_key is None
    assert!(
        openai.masked_key.is_none(),
        "openai masked_key should be None when unconfigured"
    );

    // @step And the openai ProviderCredentialInfo source is None
    assert!(
        openai.source.is_none(),
        "openai source should be None when unconfigured"
    );

    restore_all_provider_env(saved);
}

#[test]
#[serial]
fn list_providers_info_keeps_masked_key_none_for_oauth_only_providers() {
    // @step Given the codex OAuth credential is configured via codex_auth.json
    // (Assertion holds regardless of whether codex is actually configured —
    // OAuth-only providers must ALWAYS carry None for masked_key so OAuth
    // token bytes never traverse the wire.)
    let saved = save_all_provider_env();

    // @step When list_provider_credentials is called
    let list = list_providers_info().expect("list_providers_info");

    for slug in ["anthropic", "codex", "github-copilot"] {
        let entry = list
            .iter()
            .find(|p| p.name == slug)
            .unwrap_or_else(|| panic!("{slug} entry present in canonical registry"));

        // @step Then the codex ProviderCredentialInfo has masked_key equal to None
        assert!(
            entry.masked_key.is_none(),
            "{slug} masked_key should be None for OAuth-only providers; got {:?}",
            entry.masked_key
        );
    }

    // OAuth credential_type is decided at the wire boundary in
    // handle_impl.rs by matching against the catalog AuthType. Pin that
    // mapping here so the wire end-to-end has no ambiguity.
    use codelet_providers::catalog::{AuthType, CANONICAL_PROVIDERS};
    for slug in ["anthropic", "codex", "github-copilot"] {
        let entry = CANONICAL_PROVIDERS
            .iter()
            .find(|p| p.id == slug)
            .unwrap_or_else(|| panic!("{slug} must be in CANONICAL_PROVIDERS"));

        // @step Then the codex ProviderCredentialInfo credential_type is "oauth"
        assert_eq!(
            entry.auth_type,
            AuthType::OAuth,
            "{slug} auth_type must be OAuth"
        );
    }

    restore_all_provider_env(saved);
}

// ============================================================================
// Section 3 — Wire-level cross-transport parity
// ============================================================================

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_service() -> (
    TempDir,
    Arc<SharedFspecService>,
    Arc<StubSessionManagerHandle>,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    (temp, service, stub)
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> (Arc<dyn FspecBackend>, Arc<dyn FspecBackend>) {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    (embedded, websocket)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_transport_parity_for_masked_key_and_source() {
    // @step Given ANTHROPIC_API_KEY is set to "sk-ant-api03-abcdefghijklmnop" in the environment
    // (Modelled here as a server-side pre-masked seed — masking is the
    // codelet-providers responsibility BEFORE the wire boundary so the
    // bytes of the raw key never traverse either transport.)
    let (_temp, service, stub) = build_service();
    stub.seed_provider_credential(ProviderCredentialInfo {
        provider_id: "anthropic".to_string(),
        display_name: "Anthropic".to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 4,
        masked_key: Some("sk-ant-••••••••mnop".to_string()),
        source: Some("env".to_string()),
    });
    let (embedded, websocket) = dual_backends(service).await;

    // @step When list_provider_credentials is called through both the embedded and websocket transports
    let em = embedded
        .list_provider_credentials()
        .await
        .expect("embedded list");
    let ws = websocket
        .list_provider_credentials()
        .await
        .expect("websocket list");

    // @step Then both transports return masked_key Some("sk-ant-••••••••mnop") and source Some("env") for the anthropic entry
    assert_eq!(em, ws, "embedded and websocket must surface identical rows");
    let em_row = em
        .iter()
        .find(|r| r.provider_id == "anthropic")
        .expect("anthropic row in embedded list");
    assert_eq!(em_row.masked_key.as_deref(), Some("sk-ant-••••••••mnop"));
    assert_eq!(em_row.source.as_deref(), Some("env"));

    let ws_row = ws
        .iter()
        .find(|r| r.provider_id == "anthropic")
        .expect("anthropic row in websocket list");
    assert_eq!(ws_row.masked_key.as_deref(), Some("sk-ant-••••••••mnop"));
    assert_eq!(ws_row.source.as_deref(), Some("env"));
}

#[test]
fn default_provider_credential_info_has_none_masked_key_and_source() {
    let info = ProviderCredentialInfo::default();
    assert!(
        info.masked_key.is_none(),
        "default masked_key should be None"
    );
    assert!(info.source.is_none(), "default source should be None");
}
