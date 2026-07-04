//! PROV-131 — local-profile model-probe resilience outside a tokio runtime.
//!
//! Feature: spec/features/local-profile-model-probe-resilience.feature
//!
//! Regression net for the "no reactor running" panic: when a local-server
//! `openai` profile is configured, `SessionManager::list_providers` walks
//! `build_local_profile_sections -> probe_profile_models`, which bridges an
//! async `/v1/models` probe into a synchronous trait method via
//! `tokio::task::block_in_place(|| Handle::current().block_on(..))`. That
//! bridge requires a live MULTI-THREAD tokio runtime; called from a plain
//! `#[test]` (no reactor) it panicked. The fix degrades gracefully — skip the
//! probe, treat the profile as unreachable — mirroring the TS
//! `loadProfileSections` try/catch.
//!
//! These tests seed an ISOLATED `FSPEC_USER_DIR` with a local-server profile so
//! the panic reproduces deterministically without depending on the developer's
//! real `~/.fspec/fspec-config.json`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::ProviderInfo;
use codelet_sessions::SessionManager;

/// Serializes the tests that mutate the process-global `FSPEC_USER_DIR` env var
/// so a parallel test in this binary cannot observe another test's seeded
/// config. Held across the whole (synchronous) test body.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Seed a throwaway `FSPEC_USER_DIR` containing an `fspec-config.json` with one
/// local-server `openai` profile. `base_url` deliberately points at a port that
/// refuses connections so that — when a runtime IS present — the `/v1/models`
/// probe fails fast (connection refused) rather than hanging. Returns the
/// `TempDir` guard, which the caller must keep alive for the whole test body.
fn seed_local_profile(profile: &str, base_url: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    let config = format!(
        r#"{{"providers":{{"openai":{{"profiles":{{"{profile}":{{"baseUrl":"{base_url}","apiKey":null}}}}}}}}}}"#
    );
    fs::write(dir.path().join("fspec-config.json"), config).expect("write fspec-config.json");
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    dir
}

fn find_profile<'a>(providers: &'a [ProviderInfo], profile: &str) -> Option<&'a ProviderInfo> {
    providers
        .iter()
        .find(|p| p.profile_name.as_deref() == Some(profile))
}

// =============================================================================
// Scenario: list_providers does not panic when a local-server profile is
// configured and no tokio runtime is present
// =============================================================================
#[test]
fn list_providers_does_not_panic_without_runtime() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a local-server "openai" profile named "spark-local" is configured
    let _dir = seed_local_profile("spark-local", "http://127.0.0.1:1");

    // @step And list_providers is called from a plain non-tokio context
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the server builds the provider list
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle.list_providers()));

    // @step Then list_providers returns without panicking
    assert!(
        result.is_ok(),
        "PROV-131: list_providers MUST NOT panic when a local-server profile is \
         configured and no tokio runtime is present (no-reactor graceful-degradation contract)",
    );
}

// =============================================================================
// Scenario: A profile whose probe is skipped degrades to an empty unreachable
// section
// =============================================================================
#[test]
fn skipped_probe_degrades_to_empty_unreachable_section() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a local-server "openai" profile named "spark-local" with no custom models is configured
    let _dir = seed_local_profile("spark-local", "http://127.0.0.1:1");

    // @step And list_providers is called from a plain non-tokio context so the /v1/models probe is skipped
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the server builds the provider list
    let providers = handle.list_providers();

    // @step Then the "spark-local" entry has profile_name Some("spark-local")
    let section =
        find_profile(&providers, "spark-local").expect("spark-local profile section must exist");
    assert_eq!(section.profile_name.as_deref(), Some("spark-local"));

    // @step And the "spark-local" entry has an empty models list
    assert!(
        section.models.is_empty(),
        "a skipped probe with no custom models must degrade to an empty model list",
    );

    // @step And the "spark-local" entry has is_unreachable true
    assert!(
        section.is_unreachable,
        "a skipped/failed probe with no custom models must be marked unreachable",
    );

    // @step And the "spark-local" entry is still present in the list
    assert!(
        find_profile(&providers, "spark-local").is_some(),
        "the unreachable profile section must still be returned, not dropped",
    );
}

// =============================================================================
// Scenario: On a multi-thread runtime the local-server probe still runs
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn probe_still_runs_on_multi_thread_runtime() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a local-server "openai" profile named "spark-local" is configured
    let _dir = seed_local_profile("spark-local", "http://127.0.0.1:1");

    // @step And list_providers is called on a multi-thread tokio runtime
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the server builds the provider list
    let providers = handle.list_providers();

    // @step Then the "spark-local" entry is present in the list
    let section = find_profile(&providers, "spark-local")
        .expect("spark-local profile section must be present on a multi-thread runtime");
    assert_eq!(section.profile_name.as_deref(), Some("spark-local"));

    // @step And list_providers returns without panicking
    // (reaching this assertion proves the multi-thread block_in_place bridge did
    //  not panic; the port-1 probe fails fast → unreachable, but present.)
    assert!(
        section.is_unreachable,
        "the port-1 probe is refused, so the reachable-probe path marks the section unreachable",
    );
}
