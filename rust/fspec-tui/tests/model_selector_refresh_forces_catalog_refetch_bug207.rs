//! BUG-207 — App-level dispatch test for the ModelSelector refresh action.
//!
//! Feature: spec/features/the-model-selector-refresh-action-forces-a-catalog-refetch.feature
//!
//! Scenario: "The model selector refresh action forces a catalog
//! re-fetch before re-listing providers". Drives
//! `Action::RefreshModelSelector` through `App::dispatch` against the
//! shared `MockBackend`, asserting `backend.refresh_models_cache("")` is
//! awaited BEFORE `backend.list_providers()` (ordering proven via the
//! `call_log` sequence in `tests/common/mod.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::ProviderInfo;
use tokio::time::timeout;

mod common;
use common::MockBackend;

/// Drain every pending tokio task spawned by `App::dispatch` AND fold
/// any queued action_tx messages back into the App. Mirrors the helper
/// in `provider_settings_dispatch_rpc054.rs`.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn pinfo(id: &str) -> ProviderInfo {
    ProviderInfo {
        key: id.to_string(),
        display_name: id.to_string(),
        ..ProviderInfo::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn model_selector_refresh_forces_catalog_refetch_before_relisting() {
    // @step Given the model selector is open
    let mock = Arc::new(MockBackend::new());
    mock.seed_providers(vec![pinfo("openai")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::OpenModelSelectorView);
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().active_view, ViewMode::ModelSelector);
    // The open action itself lists providers once; snapshot the log so
    // the assertions below only consider calls made by the refresh.
    wait_until(
        || mock.list_providers_calls() >= 1,
        "initial list_providers from open",
    )
    .await;
    let calls_before_refresh = mock.call_log().len();
    assert_eq!(&mock.call_log()[calls_before_refresh - 1..], ["list_providers"]);

    // @step When the refresh action is triggered
    app.dispatch(Action::RefreshModelSelector);
    drain_pending(&mut app).await;
    wait_until(
        || mock.refresh_models_cache_calls() >= 1,
        "refresh_models_cache awaited",
    )
    .await;

    // @step Then the models cache is force-refreshed before the provider list is fetched
    // (ordering proven by the call_log: refresh_models_cache("") must
    // appear BEFORE the list_providers that follows it)
    let log = mock.call_log();
    let new_calls = &log[calls_before_refresh..];
    assert_eq!(
        new_calls,
        ["refresh_models_cache", "list_providers"],
        "refresh action must force-refresh the catalog BEFORE re-listing providers, got {new_calls:?}"
    );
    assert_eq!(
        mock.last_refresh_models_cache().as_deref(),
        Some(""),
        "the model selector refresh spans all providers → empty provider id"
    );
}
