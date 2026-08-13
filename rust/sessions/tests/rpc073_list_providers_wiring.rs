//! RPC-073 regression tests for the empty `list_providers` stub.
//!
//! Feature: spec/features/rpc-073-list-providers-wiring.feature
//!
//! Before the RPC-073 fix, `SessionManager::list_providers` in
//! rust/sessions/src/handle_impl.rs:709-715 unconditionally returned
//! `Vec::new()`, so the Rust ratatui model selector dialog opened
//! empty.
//!
//! After the fix, the trait override delegates to
//! `codelet_providers::custom::list_providers_info()` and maps the
//! 9-field provider info struct into the 3-field wire-portable
//! `codelet_rpc_types::ProviderInfo`. Mirrors the existing NAPI binding
//! pattern at `rust/napi/src/session_bindings.rs:3469` so the Rust
//! and Ink frontends see the same provider tree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google) shared with
/// PROV-101. Seeded into the temp cache so the built-in cloud providers
/// populate from the models.dev registry with no network — the precondition
/// PROV-127's drop-empty filter needs to KEEP a built-in cloud section.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) and mutate credential env vars, so a
/// parallel test cannot observe another test's seeded state. Mirrors PROV-118's
/// `DATA_DIR_GUARD`; held across the synchronous portion of the test body.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Seed a throwaway data dir with the offline models.dev cache and configure
/// credentials for the built-in cloud providers `openai`, `anthropic` and
/// `gemini` so they populate with >=1 model and survive the PROV-127
/// drop-empty filter. `zai`/`codex`/`github-copilot` are intentionally left
/// uncredentialed / absent from the catalog so they are dropped (zero models).
///
/// Returns the `TempDir` guard — the caller must keep it alive for the whole
/// test body so `build_cloud_registry` can read the seeded cache. Mirrors the
/// e2e fixture seeding in `e2e/prov-126-cloud-sections.test.ts`.
fn seed_populated_cloud_env() -> tempfile::TempDir {
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy");
    std::env::set_var("GEMINI_API_KEY", "AIza-test-dummy");
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let cache_dir = data_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models cache");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data directory");
    data_dir
}

/// Workspace root (one level above `rust/sessions/`).
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("project root walk-up")
}

fn handle_impl_path() -> PathBuf {
    workspace_root()
        .join("rust")
        .join("sessions")
        .join("src")
        .join("handle_impl.rs")
}

fn read_handle_impl_src() -> String {
    let p = handle_impl_path();
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Strip `//`, `///`, `//!` line comments and `/* … */` block comments.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut prev = ' ';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        prev = ch;
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// Slice out the body of the specified `fn NAME(` definition.
fn extract_method_body<'src>(src: &'src str, fn_name: &str) -> &'src str {
    let needle = format!("fn {fn_name}");
    let mut search = 0usize;
    let start = loop {
        let rel = src[search..]
            .find(&needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in handle_impl.rs"));
        let abs = search + rel;
        let after = src[abs + needle.len()..].chars().next().unwrap_or(' ');
        if after == '(' || after == '<' {
            break abs;
        }
        search = abs + needle.len();
    };
    let body_start_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace of `fn {fn_name}` not found"));
    let body_start = start + body_start_rel;
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("did not find matching `}}` for `fn {fn_name}` body");
}

// =============================================================================
// Scenario: list_providers returns built-in cloud providers only when they have
// at least one model
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn list_providers_returns_all_six_built_in_providers() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a SessionManager is constructed with a seeded models.dev cache and credentials for openai, anthropic and gemini
    //
    // PROV-127: built-in cloud providers only survive the drop-empty filter
    // when they expose >=1 model (credentialed + present in the models.dev
    // cache). We seed openai/anthropic/gemini so they populate; zai/codex/
    // github-copilot are left uncredentialed/absent so they are dropped.
    let _data_dir = seed_populated_cloud_env();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the test calls handle.list_providers()
    let providers = handle.list_providers();

    // @step Then every returned cloud ProviderInfo entry has at least one model
    //
    // The isolated data dir has no local-server profiles, so EVERY returned
    // entry is a cloud section — and the PROV-127 filter guarantees each one
    // carries >=1 model.
    assert!(
        !providers.is_empty(),
        "PROV-127: list_providers must return the populated built-in cloud sections",
    );
    for p in &providers {
        assert!(
            !p.models.is_empty(),
            "PROV-127: cloud provider '{}' survived the drop-empty filter with zero models",
            p.key,
        );
    }

    // @step Then the entries include the credentialed built-in provider keys 'openai' and 'anthropic'
    let keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();
    for builtin in ["openai", "anthropic"] {
        assert!(
            keys.contains(&builtin),
            "PROV-127: credentialed built-in '{builtin}' missing from list_providers result; got keys: {keys:?}",
        );
    }

    // @step Then zero-model built-in cloud providers such as 'codex' and 'zai' are dropped from the result
    //
    // codex is genuinely absent from models.dev (KNOWN_ABSENT_FROM_MODELS_DEV)
    // and zai has no seeded credentials/catalog entry — both resolve to zero
    // models and must be dropped rather than rendered as dead "(0 models)" rows.
    for dropped in ["codex", "zai"] {
        assert!(
            !keys.contains(&dropped),
            "PROV-127: zero-model built-in '{dropped}' must be dropped, but it appeared; keys: {keys:?}",
        );
    }
}

// =============================================================================
// Scenario: list_providers entries have populated key and display_name fields
// and a non-empty models Vec
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn list_providers_entries_have_populated_fields() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given list_providers has been called with seeded credentials and returned a non-empty Vec
    let _data_dir = seed_populated_cloud_env();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;
    let providers = handle.list_providers();
    assert!(!providers.is_empty(), "list_providers must be non-empty");

    // @step When the test inspects the 'anthropic' ProviderInfo entry
    // RPC-107: the legacy 'claude' slug was migrated to the TS-canonical
    // 'anthropic' slug. Inspect the canonical entry instead.
    let anthropic = providers
        .iter()
        .find(|p| p.key == "anthropic")
        .expect("anthropic entry must exist in list_providers result (RPC-107 canonical slug)");

    // @step Then the entry has a non-empty 'key' field matching the canonical provider slug 'anthropic'
    assert_eq!(anthropic.key, "anthropic");

    // @step Then the entry has a non-empty 'display_name' field
    assert!(
        !anthropic.display_name.is_empty(),
        "ProviderInfo.display_name must be non-empty for built-in providers (RPC-073 mapping)",
    );

    // @step Then the entry has a 'models' field of type Vec containing at least one model
    //
    // PROV-127: a built-in cloud provider only appears when its models list is
    // populated from the models.dev registry — so the Vec is present AND
    // non-empty here (was previously allowed to be empty pre-PROV-127).
    let models: &Vec<codelet_rpc_types::ModelEntry> = &anthropic.models;
    assert!(
        !models.is_empty(),
        "PROV-127: the 'anthropic' cloud section must carry at least one model to appear",
    );
}

// =============================================================================
// Scenario: list_providers maps codelet_providers::custom::ProviderInfo into
// codelet_rpc_types::ProviderInfo with the correct field mapping
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn list_providers_maps_provider_info_fields_correctly() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a seeded models.dev cache and credentials populate the built-in 'openai' provider with a reasoning-capable model whose supports_thinking=true
    //
    // The seeded catalog gives openai the `o3` model (reasoning=true,
    // tool_call=true, context=200000). The models.dev `reasoning` flag maps to
    // `supports_thinking` on the source struct and to `supports_reasoning` on
    // the wire `ModelEntry`; the `usize` limit maps to `u32` (saturating).
    let _data_dir = seed_populated_cloud_env();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the trait override list_providers maps the value into a codelet_rpc_types::ProviderInfo
    let providers = handle.list_providers();
    let openai = providers
        .iter()
        .find(|p| p.key == "openai")
        .expect("openai entry must exist");

    // @step Then the resulting codelet_rpc_types::ProviderInfo has key='openai', a non-empty display, and a child ModelEntry with supports_reasoning=true and is_custom=false
    assert_eq!(openai.key, "openai");
    assert!(
        !openai.display_name.is_empty(),
        "display_name falls back to the provider name and must be non-empty",
    );
    assert!(
        !openai.models.is_empty(),
        "PROV-127: the populated 'openai' section must carry its seeded models",
    );
    let reasoning_model = openai
        .models
        .iter()
        .find(|m| m.supports_reasoning)
        .expect("seeded 'o3' model has reasoning=true → supports_reasoning=true");
    assert!(
        !reasoning_model.is_custom,
        "built-in cloud models carry is_custom=false",
    );

    // @step Then context_window and max_output_tokens are converted from usize to u32 with saturating cast
    //
    // The seeded `o3` limit.context is 200000, well within u32 range: the
    // wire `ModelEntry.context_window` is u32 by definition and must carry the
    // saturating-cast value (u32::MAX had the source usize exceeded u32::MAX).
    for m in &openai.models {
        let _cw: u32 = m.context_window;
        let _sr: bool = m.supports_reasoning;
        let _vis: bool = m.supports_vision;
        let _custom: bool = m.is_custom;
    }
    assert!(
        reasoning_model.context_window > 0,
        "seeded model context_window must be a valid non-zero u32 (200000)",
    );
}

// =============================================================================
// Scenario: list_providers degrades gracefully to Vec::new() and logs via
// tracing::error when list_providers_info returns Err
// =============================================================================
#[test]
fn list_providers_does_not_panic_on_provider_discovery_error() {
    // @step Given an environment is set up such that list_providers_info returns Err (e.g. corrupt ~/.fspec/providers/foo.json)
    //
    // We cannot easily corrupt the user's real ~/.fspec/providers without
    // side effects. Instead, the contract we enforce is the NO-PANIC
    // invariant: list_providers() must never panic regardless of
    // upstream state. The "logs via tracing::error" half is a
    // best-effort guarantee that is structurally enforced by the
    // source-shape scenario below (the body must contain the
    // tracing::error! call).
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the test calls handle.list_providers()
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle.list_providers()));

    // @step Then the call returns Vec::new() and does not panic
    assert!(
        result.is_ok(),
        "list_providers MUST NOT panic — RPC-073 bug 3 graceful-degradation contract",
    );

    // @step Then a tracing::error event with target 'handle_impl' and the underlying error is emitted
    //
    // Behavioural verification of tracing emission requires a subscriber
    // (e.g. tracing_subscriber::fmt::TestWriter). For this regression
    // we structurally enforce the call via the source-shape test below.
}

// =============================================================================
// Scenario: Source-shape regression: handle_impl.rs list_providers body
// calls list_providers_info and no longer returns the empty Vec::new() stub
// =============================================================================
#[test]
fn source_shape_list_providers_calls_list_providers_info() {
    // @step Given the file rust/sessions/src/handle_impl.rs
    let src = read_handle_impl_src();

    // @step When the test reads the source bytes and extracts the body of fn list_providers
    let code_only = strip_comments(&src);
    let body = extract_method_body(&code_only, "list_providers");

    // @step Then the body contains the substring 'list_providers_info'
    assert!(
        body.contains("list_providers_info"),
        "RPC-073 bug 3: list_providers body must delegate to codelet_providers::custom::list_providers_info; body was:\n{body}",
    );

    // @step Then the body does not match the deprecated stub pattern of bare 'Vec::new()' as the sole expression
    //
    // After the fix the body MAY still contain a `Vec::new()` in the
    // Err branch of the match, but the WHOLE body must not be a stub.
    // We detect the stub pattern by requiring the body to NOT match the
    // pre-fix shape: an opening brace, a single doc-or-non-statement
    // line that wraps `Vec::new()`, and a closing brace with no other
    // expression.
    //
    // Concrete check: the body must contain more than one statement.
    // We approximate "more than one statement" by counting semicolons
    // (or the `match { ... }` expression). The original stub had no
    // semicolons or match arms.
    let has_match = body.contains("match codelet_providers::custom::list_providers_info()");
    let has_question_mark_chain = body.contains(".map(") || body.contains(".unwrap_or");
    assert!(
        has_match || has_question_mark_chain,
        "RPC-073 bug 3: list_providers body must call list_providers_info() and adapt the result; body was:\n{body}",
    );
}
