//! RPC-107 — Integration tests: list_provider_credentials returns the
//! canonical 17-provider ordered list with TS-canonical display names.
//!
//! Feature: spec/features/provider-catalog-canonical-17-provider-ordered-list-with-display-names.feature
//!
//! Drives the real `codelet_sessions::SessionManager::list_provider_credentials`
//! (which delegates to `codelet_providers::custom::list_providers_info`)
//! through controlled env-var + HOME redirection so the response is
//! deterministic.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::catalog::{AuthType, CanonicalProvider, CANONICAL_PROVIDERS};
use codelet_rpc_types::ProviderCredentialInfo;
use codelet_sessions::SessionManager;
use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;

// =========================================================================
// Unit tests — CANONICAL_PROVIDERS static slice surface (no I/O)
// =========================================================================

/// Helper: TS-canonical ordered list of (id, display_name) pairs derived
/// from src/utils/provider-registry.ts.
fn ts_canonical_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("openai", "OpenAI API"),
        ("anthropic", "Anthropic"),
        ("cohere", "Cohere"),
        ("gemini", "Google Gemini"),
        ("mistral", "Mistral AI"),
        ("xai", "xAI"),
        ("together", "Together AI"),
        ("huggingface", "Hugging Face"),
        ("openrouter", "OpenRouter"),
        ("groq", "Groq"),
        ("deepseek", "DeepSeek"),
        ("moonshot", "Moonshot"),
        ("galadriel", "Galadriel"),
        ("azure", "Azure OpenAI"),
        ("zai", "Z.AI"),
        ("codex", "Codex (ChatGPT)"),
        ("github-copilot", "GitHub Copilot"),
    ]
}

// =============================================================================
// Scenario: CANONICAL_PROVIDERS slice declares exactly 17 entries in the TS-canonical order
// =============================================================================
#[test]
fn canonical_providers_slice_has_seventeen_entries_in_ts_canonical_order() {
    // @step Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    let slice: &[CanonicalProvider] = CANONICAL_PROVIDERS;

    // @step When the slice is iterated in declaration order
    let ids: Vec<&str> = slice.iter().map(|p| p.id).collect();

    // @step Then it yields exactly 17 entries
    assert_eq!(slice.len(), 17, "RPC-107: CANONICAL_PROVIDERS must declare exactly 17 entries; got {}", slice.len());

    // @step And the provider ids in order are "openai", "anthropic", "cohere", "gemini", "mistral", "xai", "together", "huggingface", "openrouter", "groq", "deepseek", "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot"
    let expected: Vec<&str> = ts_canonical_pairs().iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, expected, "RPC-107: CANONICAL_PROVIDERS order must match TS SUPPORTED_PROVIDERS");
}

// =============================================================================
// Scenario: CANONICAL_PROVIDERS display names match the TS PROVIDER_REGISTRY byte-for-byte
// =============================================================================
#[test]
fn canonical_providers_display_names_match_ts_registry_byte_for_byte() {
    // @step Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    let slice: &[CanonicalProvider] = CANONICAL_PROVIDERS;

    // @step When the display_name field is read from each entry in order
    let names: Vec<&str> = slice.iter().map(|p| p.display_name).collect();

    // @step Then the display names in order are "OpenAI API", "Anthropic", "Cohere", "Google Gemini", "Mistral AI", "xAI", "Together AI", "Hugging Face", "OpenRouter", "Groq", "DeepSeek", "Moonshot", "Galadriel", "Azure OpenAI", "Z.AI", "Codex (ChatGPT)", "GitHub Copilot"
    let expected: Vec<&str> = ts_canonical_pairs().iter().map(|(_, n)| *n).collect();
    assert_eq!(names, expected, "RPC-107: CANONICAL_PROVIDERS display_name strings must match TS PROVIDER_REGISTRY byte-for-byte");
}

// =============================================================================
// Scenario: CANONICAL_PROVIDERS tags codex, anthropic, and github-copilot as OAuth auth_type
// =============================================================================
#[test]
fn canonical_providers_marks_anthropic_codex_and_copilot_as_oauth() {
    // @step Given the codelet-providers crate exports a static CANONICAL_PROVIDERS slice
    let slice: &[CanonicalProvider] = CANONICAL_PROVIDERS;

    // @step When the auth_type field is read from each entry
    let oauth_ids: Vec<&str> = slice.iter()
        .filter(|p| matches!(p.auth_type, AuthType::OAuth))
        .map(|p| p.id)
        .collect();
    let api_key_ids: Vec<&str> = slice.iter()
        .filter(|p| matches!(p.auth_type, AuthType::ApiKey))
        .map(|p| p.id)
        .collect();

    // @step Then the entries with id "anthropic", "codex", and "github-copilot" have auth_type AuthType::OAuth
    let mut sorted_oauth = oauth_ids.clone();
    sorted_oauth.sort();
    assert_eq!(sorted_oauth, vec!["anthropic", "codex", "github-copilot"],
        "RPC-107: OAuth auth_type set must equal {{anthropic, codex, github-copilot}}; got {oauth_ids:?}");

    // @step And every other entry has auth_type AuthType::ApiKey
    assert_eq!(api_key_ids.len(), 14, "RPC-107: 14 api-key providers expected; got {}", api_key_ids.len());
    for forbidden in ["anthropic", "codex", "github-copilot"] {
        assert!(!api_key_ids.contains(&forbidden), "RPC-107: '{forbidden}' must NOT be in the ApiKey bucket");
    }
}

// =========================================================================
// Test fixtures (RAII env + cwd guards mirroring custom_provider_manager_integration_test)
// =========================================================================

struct EnvGuard {
    key: String,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key: key.to_string(), prior }
    }

    fn set_path(key: &str, value: &Path) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key: key.to_string(), prior }
    }

    fn remove(key: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key: key.to_string(), prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prior.take() {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

struct CwdGuard {
    prior: PathBuf,
}

impl CwdGuard {
    fn set(new_cwd: &Path) -> Self {
        let prior = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(new_cwd).expect("set cwd");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// Combined fixture redirecting HOME + FSPEC_HOME + CWD so neither
/// `~/.fspec/providers/` nor `<cwd>/.fspec/providers/` discovery reads
/// the user's real filesystem.
struct DiscoveryFixture {
    _home_tmp: TempDir,
    project_tmp: TempDir,
    _home_guard: EnvGuard,
    _fspec_guard: EnvGuard,
    _cwd_guard: CwdGuard,
    _env_guards: Vec<EnvGuard>,
}

impl DiscoveryFixture {
    fn new() -> Self {
        let home_tmp = TempDir::new().expect("home tempdir");
        let project_tmp = TempDir::new().expect("project tempdir");
        let fspec_dir = home_tmp.path().join(".fspec");
        let credentials_dir = fspec_dir.join("credentials");
        fs::create_dir_all(&credentials_dir).unwrap();
        let home_guard = EnvGuard::set_path("HOME", home_tmp.path());
        let fspec_guard = EnvGuard::set_path("FSPEC_HOME", &credentials_dir);
        let cwd_guard = CwdGuard::set(project_tmp.path());

        // Clear ALL provider env vars so detection is deterministic.
        // Tests that need a specific env var add their own EnvGuard::set
        // AFTER construction.
        let mut env_guards = Vec::new();
        for var in [
            "ANTHROPIC_API_KEY",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "OPENAI_API_KEY",
            "GOOGLE_GENERATIVE_AI_API_KEY",
            "ZAI_API_KEY",
            "ZAI_PLAN_API_KEY",
            "COHERE_API_KEY",
            "MISTRAL_API_KEY",
            "XAI_API_KEY",
            "TOGETHER_API_KEY",
            "HF_TOKEN",
            "OPENROUTER_API_KEY",
            "GROQ_API_KEY",
            "DEEPSEEK_API_KEY",
            "MOONSHOT_API_KEY",
            "GALADRIEL_API_KEY",
            "AZURE_OPENAI_API_KEY",
        ] {
            env_guards.push(EnvGuard::remove(var));
        }

        Self {
            _home_tmp: home_tmp,
            project_tmp,
            _home_guard: home_guard,
            _fspec_guard: fspec_guard,
            _cwd_guard: cwd_guard,
            _env_guards: env_guards,
        }
    }

    fn project_root(&self) -> &Path {
        self.project_tmp.path()
    }
}

fn write_project_custom_provider(project_root: &Path, name: &str, env_var: &str) {
    let providers_dir = project_root.join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).unwrap();
    let obj = json!({
        "name": name,
        "display_name": format!("Custom {name}"),
        "base_url": "http://localhost:9999/v1",
        "facade": "openai",
        "api_key_env_var": env_var,
        "models": {
            "my-model": { "id": "my-model" }
        }
    });
    fs::write(
        providers_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&obj).unwrap(),
    )
    .unwrap();
}

fn handle() -> Arc<dyn SessionManagerHandle> {
    Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>
}

/// TS-canonical ordering for assertions.
const CANONICAL_ORDER: [&str; 17] = [
    "openai", "anthropic", "cohere", "gemini", "mistral", "xai",
    "together", "huggingface", "openrouter", "groq", "deepseek",
    "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot",
];

const CANONICAL_DISPLAY_NAMES: [&str; 17] = [
    "OpenAI API", "Anthropic", "Cohere", "Google Gemini", "Mistral AI",
    "xAI", "Together AI", "Hugging Face", "OpenRouter", "Groq",
    "DeepSeek", "Moonshot", "Galadriel", "Azure OpenAI", "Z.AI",
    "Codex (ChatGPT)", "GitHub Copilot",
];

// =============================================================================
// Scenario: Empty workspace returns 17 canonical rows in order with canonical display names
// =============================================================================
#[test]
#[serial]
fn empty_workspace_returns_seventeen_canonical_rows_in_order() {
    // @step Given no provider env vars are set in the process environment
    // @step And no custom provider configs exist on disk
    let _fx = DiscoveryFixture::new();

    // @step When list_provider_credentials is called
    let list = handle().list_provider_credentials();

    // @step Then the response contains exactly 17 ProviderCredentialInfo entries
    assert_eq!(
        list.len(),
        17,
        "RPC-107: empty workspace must yield exactly 17 canonical rows; got {}",
        list.len()
    );

    // @step And the entries appear in canonical order with provider_id "openai", "anthropic", "cohere", "gemini", "mistral", "xai", "together", "huggingface", "openrouter", "groq", "deepseek", "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot"
    let ids: Vec<&str> = list.iter().map(|e| e.provider_id.as_str()).collect();
    assert_eq!(ids, CANONICAL_ORDER.to_vec(),
        "RPC-107: provider_id order must match TS SUPPORTED_PROVIDERS");

    // @step And every entry has display_name set to the TS-canonical display string
    let names: Vec<&str> = list.iter().map(|e| e.display_name.as_str()).collect();
    assert_eq!(names, CANONICAL_DISPLAY_NAMES.to_vec(),
        "RPC-107: display_name strings must match TS PROVIDER_REGISTRY byte-for-byte");

    // @step And every entry has configured == false
    for entry in &list {
        assert!(
            !entry.configured,
            "RPC-107: entry {} must be unconfigured in an empty workspace; was configured=true",
            entry.provider_id
        );
    }
}

// =============================================================================
// Scenario: ANTHROPIC_API_KEY alone marks the Anthropic row configured under slug "anthropic"
// =============================================================================
#[test]
#[serial]
fn anthropic_api_key_marks_anthropic_row_configured_under_anthropic_slug() {
    // @step Given the env var ANTHROPIC_API_KEY is set to "sk-ant-test"
    // @step And no other provider env vars are set
    // @step And no custom provider configs exist on disk
    let _fx = DiscoveryFixture::new();
    let _anthropic = EnvGuard::set("ANTHROPIC_API_KEY", "sk-ant-test");

    // @step When list_provider_credentials is called
    let list = handle().list_provider_credentials();

    // @step Then the response contains exactly 17 entries in canonical order
    assert_eq!(list.len(), 17, "RPC-107: must yield 17 canonical rows");
    let ids: Vec<&str> = list.iter().map(|e| e.provider_id.as_str()).collect();
    assert_eq!(ids, CANONICAL_ORDER.to_vec(),
        "RPC-107: provider_id order must match TS canon");

    // @step And the entry at index 1 has provider_id "anthropic" and display_name "Anthropic" and configured == true
    let anthropic = &list[1];
    assert_eq!(anthropic.provider_id, "anthropic",
        "RPC-107: index 1 must carry the canonical 'anthropic' slug");
    assert_eq!(anthropic.display_name, "Anthropic",
        "RPC-107: Anthropic display_name must be 'Anthropic'");
    assert!(anthropic.configured,
        "RPC-107: Anthropic row must be configured when ANTHROPIC_API_KEY is set");

    // @step And no entry has provider_id "claude"
    assert!(
        !ids.contains(&"claude"),
        "RPC-107: the legacy 'claude' slug must NOT appear in the wire response; got {ids:?}"
    );

    // @step And every other entry has configured == false
    for (i, entry) in list.iter().enumerate() {
        if i == 1 {
            continue;
        }
        assert!(
            !entry.configured,
            "RPC-107: entry {} (idx {i}) must be unconfigured; was configured=true",
            entry.provider_id
        );
    }
}

// =============================================================================
// Scenario: Canonical rows precede custom providers in the response
// =============================================================================
#[test]
#[serial]
fn canonical_rows_precede_custom_providers() {
    // @step Given the env var ANTHROPIC_API_KEY is set
    // @step And the env var GROQ_API_KEY is set
    // @step And the env var OPENROUTER_API_KEY is set
    // @step And a custom provider config "my-vllm" exists on disk
    let fx = DiscoveryFixture::new();
    let _ant = EnvGuard::set("ANTHROPIC_API_KEY", "sk-ant-1");
    let _groq = EnvGuard::set("GROQ_API_KEY", "gsk_1");
    let _orouter = EnvGuard::set("OPENROUTER_API_KEY", "sk-or-1");
    let _vllm_env = EnvGuard::set("MY_VLLM_API_KEY", "vllm-key");
    write_project_custom_provider(fx.project_root(), "my-vllm", "MY_VLLM_API_KEY");

    // @step When list_provider_credentials is called
    let list = handle().list_provider_credentials();

    // @step Then the response contains exactly 18 entries
    assert_eq!(
        list.len(),
        18,
        "RPC-107: 17 canonical + 1 custom = 18 rows; got {}",
        list.len()
    );

    // @step And the first 17 entries are the canonical providers in canonical order
    let canonical_ids: Vec<&str> = list[..17].iter().map(|e| e.provider_id.as_str()).collect();
    assert_eq!(canonical_ids, CANONICAL_ORDER.to_vec(),
        "RPC-107: first 17 entries must be the canonical providers in canonical order");

    // @step And the entry at index 17 has provider_id "my-vllm"
    assert_eq!(list[17].provider_id, "my-vllm",
        "RPC-107: custom provider must be appended AFTER the canonical 17");

    // @step And the entries with provider_id "anthropic", "groq", and "openrouter" have configured == true
    for target in ["anthropic", "groq", "openrouter"] {
        let entry = list.iter()
            .find(|e| e.provider_id == target)
            .unwrap_or_else(|| panic!("RPC-107: '{target}' missing from response"));
        assert!(entry.configured,
            "RPC-107: '{target}' row must be configured when its env var is set");
    }
}

// =============================================================================
// Scenario: display_name on every canonical entry is sourced from the catalog not the slug
// =============================================================================
#[test]
#[serial]
fn display_name_on_every_canonical_entry_comes_from_catalog_not_slug() {
    // @step Given no provider env vars are set in the process environment
    let _fx = DiscoveryFixture::new();

    // @step When list_provider_credentials is called
    let list = handle().list_provider_credentials();

    // @step Then for every canonical entry the display_name differs from the provider_id where the TS canon differs
    let expected: Vec<(&str, &str)> = CANONICAL_ORDER
        .iter()
        .copied()
        .zip(CANONICAL_DISPLAY_NAMES.iter().copied())
        .collect();
    for (id, expected_name) in &expected {
        let entry = list.iter()
            .find(|e| e.provider_id == *id)
            .unwrap_or_else(|| panic!("RPC-107: '{id}' missing from response"));
        assert_eq!(&entry.display_name, expected_name,
            "RPC-107: '{id}' display_name must be '{expected_name}', got '{}'",
            entry.display_name);
    }

    // @step And the entry with provider_id "openai" has display_name "OpenAI API"
    let openai = list.iter().find(|e| e.provider_id == "openai").unwrap();
    assert_eq!(openai.display_name, "OpenAI API");

    // @step And the entry with provider_id "gemini" has display_name "Google Gemini"
    let gemini = list.iter().find(|e| e.provider_id == "gemini").unwrap();
    assert_eq!(gemini.display_name, "Google Gemini");

    // @step And the entry with provider_id "github-copilot" has display_name "GitHub Copilot"
    let copilot = list.iter().find(|e| e.provider_id == "github-copilot").unwrap();
    assert_eq!(copilot.display_name, "GitHub Copilot");

    // @step And the entry with provider_id "azure" has display_name "Azure OpenAI"
    let azure = list.iter().find(|e| e.provider_id == "azure").unwrap();
    assert_eq!(azure.display_name, "Azure OpenAI");

    // @step And the entry with provider_id "codex" has display_name "Codex (ChatGPT)"
    let codex = list.iter().find(|e| e.provider_id == "codex").unwrap();
    assert_eq!(codex.display_name, "Codex (ChatGPT)");
}

// Silence unused warning for the type alias when only a subset is invoked.
fn _assert_type_shape(_: ProviderCredentialInfo) {}

// =========================================================================
// Cross-transport parity tests (RPC-107)
// =========================================================================
//
// These tests drive the REAL `SessionManager` (which delegates to
// `codelet_providers::custom::list_providers_info`) through both an
// `EmbeddedFspecBackend` and a `WebSocketFspecBackend`, asserting that
// the canonical 17 rows arrive identically across transports.

use codelet_core::session_manager_handle::SessionManagerHandle as DynHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_real_service() -> (TempDir, Arc<SharedFspecService>) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let session_manager = Arc::new(SessionManager::new());
    let handle: Arc<dyn DynHandle> = session_manager;
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd),
    );
    (temp, service)
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

// =============================================================================
// Scenario: Embedded and WebSocket transports surface the same 17 canonical rows in the same order
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn embedded_and_websocket_surface_identical_canonical_seventeen_rows() {
    // @step Given a SharedFspecService backed by a StubSessionManagerHandle
    //
    // (RPC-107 contract: the CANONICAL_PROVIDERS slice lives below the
    // wire boundary inside codelet-providers, so we use the REAL
    // SessionManager here rather than the stub — the stub bypasses
    // list_providers_info and would not exercise the slice.)
    let _fx = DiscoveryFixture::new();
    let (_temp, service) = build_real_service();

    // @step And both an EmbeddedFspecBackend and a WebSocketFspecBackend over that service
    let (embedded, websocket) = dual_backends(service).await;

    // @step When list_provider_credentials is called via the embedded transport
    let em_list = embedded
        .list_provider_credentials()
        .await
        .expect("embedded list");

    // @step And list_provider_credentials is called via the WebSocket transport
    let ws_list = websocket
        .list_provider_credentials()
        .await
        .expect("websocket list");

    // @step Then both responses contain exactly 17 entries
    assert_eq!(em_list.len(), 17,
        "RPC-107: embedded transport must surface 17 canonical rows; got {}", em_list.len());
    assert_eq!(ws_list.len(), 17,
        "RPC-107: websocket transport must surface 17 canonical rows; got {}", ws_list.len());

    // @step And both responses list the canonical provider_ids in identical canonical order
    let em_ids: Vec<&str> = em_list.iter().map(|e| e.provider_id.as_str()).collect();
    let ws_ids: Vec<&str> = ws_list.iter().map(|e| e.provider_id.as_str()).collect();
    assert_eq!(em_ids, CANONICAL_ORDER.to_vec(),
        "RPC-107: embedded provider_id order must match TS canon");
    assert_eq!(ws_ids, CANONICAL_ORDER.to_vec(),
        "RPC-107: websocket provider_id order must match TS canon");
    assert_eq!(em_ids, ws_ids,
        "RPC-107: both transports must surface identical canonical order");

    // @step And both responses list the canonical display_names in identical canonical order
    let em_names: Vec<&str> = em_list.iter().map(|e| e.display_name.as_str()).collect();
    let ws_names: Vec<&str> = ws_list.iter().map(|e| e.display_name.as_str()).collect();
    assert_eq!(em_names, CANONICAL_DISPLAY_NAMES.to_vec(),
        "RPC-107: embedded display_name must match TS PROVIDER_REGISTRY");
    assert_eq!(ws_names, CANONICAL_DISPLAY_NAMES.to_vec(),
        "RPC-107: websocket display_name must match TS PROVIDER_REGISTRY");
    assert_eq!(em_names, ws_names,
        "RPC-107: both transports must surface identical display_name strings");
}
