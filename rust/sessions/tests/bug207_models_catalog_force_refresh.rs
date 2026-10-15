//! BUG-207 — force-refresh of the models.dev catalog through the
//! `SessionManagerHandle::refresh_models_cache` surface.
//!
//! Feature: spec/features/refreshing-the-models-cache-force-fetches-the-models-dev-catalog.feature
//!
//! Scenarios:
//! - "Refreshing the models cache force-fetches the catalog and returns
//!   the provider's models"
//! - "Refreshing with no provider refreshes the shared catalog only"
//!
//! Setup: a throwaway data dir seeded with a MONTHS-OLD models.dev
//! snapshot (missing the newly released model), plus a hand-written
//! HTTP/1.1 mock serving the NEW catalog and counting hits. The catalog
//! endpoint is redirected via the `FSPEC_MODELS_DEV_URL` seam in
//! `ModelCache::new()` (the cache PATH already follows the data dir via
//! `codelet_common::set_data_directory`).
//!
//! Red-phase rationale: today `handle_impl::refresh_models_cache` never
//! touches the network (it re-lists static provider info), so the mock
//! hit count stays 0 and the anthropic assertion finds no new model —
//! both tests fail for exactly the bug's reason.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// DATA_DIR_GUARD serialises env-var-touching tests; std::sync::Mutex held
// across awaits is intentional (mirrors rpc073_list_providers_wiring.rs).
#![allow(clippy::await_holding_lock)]

use std::fs::{self, File};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

/// Serializes the tests that swap the process-global data directory and
/// mutate `FSPEC_MODELS_DEV_URL` / credential env vars. Mirrors the
/// `DATA_DIR_GUARD` pattern from `rpc073_list_providers_wiring.rs`.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Months-old snapshot: anthropic lists ONLY the previous-generation
/// model. The newly released `claude-opus-5` is absent — that is the
/// whole point of the bug.
const OLD_CATALOG: &str = r#"{
    "anthropic": {
        "id": "anthropic",
        "name": "Anthropic",
        "env": ["ANTHROPIC_API_KEY"],
        "models": {
            "claude-opus-4-5": {
                "id": "claude-opus-4-5",
                "name": "Claude Opus 4.5",
                "release_date": "2025-11-01",
                "reasoning": true,
                "tool_call": true,
                "limit": {"context": 200000, "output": 64000}
            }
        }
    }
}"#;

/// Fresh catalog served by the mock: adds the newly released model.
const NEW_CATALOG: &str = r#"{
    "anthropic": {
        "id": "anthropic",
        "name": "Anthropic",
        "env": ["ANTHROPIC_API_KEY"],
        "models": {
            "claude-opus-4-5": {
                "id": "claude-opus-4-5",
                "name": "Claude Opus 4.5",
                "release_date": "2025-11-01",
                "reasoning": true,
                "tool_call": true,
                "limit": {"context": 200000, "output": 64000}
            },
            "claude-opus-5": {
                "id": "claude-opus-5",
                "name": "Claude Opus 5",
                "release_date": "2026-09-01",
                "reasoning": true,
                "tool_call": true,
                "limit": {"context": 1000000, "output": 128000}
            }
        }
    }
}"#;

/// Seed a throwaway data dir whose `cache/models.json` holds the OLD
/// catalog with an mtime months in the past. The cache PATH derives from
/// the global data dir, so no path seam is needed — only the URL seam.
fn seed_stale_data_dir() -> tempfile::TempDir {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let cache_dir = data_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    let snapshot = cache_dir.join("models.json");
    fs::write(&snapshot, OLD_CATALOG).expect("write stale models cache");

    // Backdate the mtime ~180 days so the snapshot is unambiguously
    // older than DEFAULT_CACHE_TTL (24h) — "months old" per the scenario.
    let months_ago = std::time::SystemTime::now()
        - std::time::Duration::from_secs(180 * 24 * 60 * 60);
    let f = File::options()
        .write(true)
        .open(&snapshot)
        .expect("open snapshot for mtime update");
    f.set_modified(months_ago).expect("backdate snapshot mtime");

    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("set data directory");
    data_dir
}

/// Hand-written HTTP/1.1 mock (sessions has no axum/wiremock dev-dep).
/// Serves `NEW_CATALOG` on every GET and counts requests. Returns the
/// base URL and the hit counter.
async fn start_mock_catalog_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock server");
    let addr = listener.local_addr().expect("mock server addr");
    let hits = Arc::new(AtomicUsize::new(0));

    let hits_task = Arc::clone(&hits);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            hits_task.fetch_add(1, Ordering::SeqCst);

            // Drain the request head (we don't inspect it).
            let mut buf = [0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;

            let body = NEW_CATALOG;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });

    (format!("http://{addr}"), hits)
}

/// Scenario: "Refreshing the models cache force-fetches the catalog and
/// returns the provider's models".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_models_cache_force_fetches_catalog_and_returns_provider_models() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the models.dev snapshot on disk is months old and lacks a newly released model for a credentialed built-in provider
    let _data_dir = seed_stale_data_dir();
    let (mock_url, hits) = start_mock_catalog_server().await;
    std::env::set_var("FSPEC_MODELS_DEV_URL", &mock_url);
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-bug207-dummy");

    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the models cache is refreshed for that provider
    let result = handle.refresh_models_cache("anthropic");

    // @step Then the catalog is re-fetched from models.dev
    let entries = result.expect("refresh_models_cache must succeed against the mock catalog");
    assert!(
        hits.load(Ordering::SeqCst) >= 1,
        "BUG-207: refresh_models_cache must force a models.dev re-fetch; mock was hit {} times",
        hits.load(Ordering::SeqCst)
    );

    // @step And the returned model entries include the newly released model
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert!(
        ids.contains(&"claude-opus-5"),
        "BUG-207: the freshly released model must appear after a forced refresh; got: {ids:?}",
    );
    for e in entries.iter().filter(|e| e.id == "claude-opus-5") {
        assert!(!e.is_custom, "claude-opus-5 is a built-in cloud model entry");
    }

    // The on-disk snapshot is updated by the forced fetch, so the NEXT
    // read (without any refresh) would see the new model too.
    let snapshot = codelet_common::get_data_dir()
        .expect("data dir")
        .join("cache")
        .join("models.json");
    let on_disk = fs::read_to_string(&snapshot)
        .expect("read refreshed snapshot");
    assert!(
        on_disk.contains("claude-opus-5"),
        "the forced refresh must persist the new snapshot to disk"
    );

    std::env::remove_var("FSPEC_MODELS_DEV_URL");
    std::env::remove_var("ANTHROPIC_API_KEY");
}

/// Scenario: "Refreshing with no provider refreshes the shared catalog
/// only".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_models_cache_with_empty_provider_refreshes_shared_catalog_only() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given the models.dev snapshot on disk is months old
    let _data_dir = seed_stale_data_dir();
    let (mock_url, hits) = start_mock_catalog_server().await;
    std::env::set_var("FSPEC_MODELS_DEV_URL", &mock_url);

    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When the models cache is refreshed with an empty provider id
    let result = handle.refresh_models_cache("");

    // @step Then the catalog is re-fetched from models.dev
    let entries = result.expect("refresh with empty provider id must succeed");
    assert!(
        hits.load(Ordering::SeqCst) >= 1,
        "BUG-207: an empty-provider refresh must still re-fetch the shared catalog; mock was hit {} times",
        hits.load(Ordering::SeqCst)
    );

    // @step And no provider-specific model entries are returned
    assert!(
        entries.is_empty(),
        "an empty provider id refreshes the shared catalog only; got {} entries",
        entries.len()
    );

    std::env::remove_var("FSPEC_MODELS_DEV_URL");
}
