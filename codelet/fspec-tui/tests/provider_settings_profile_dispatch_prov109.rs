//! PROV-109 — App-level dispatch tests for the profile write surface
//! (SaveProfile / DeleteProfile / ConfirmDeleteProfile).
//!
//! Feature: spec/features/provider-config-profile-dispatch.feature
//!
//! Drives the `App::dispatch` state machine through the new profile write
//! actions against the shared `MockBackend` (per-call counters +
//! last-capture + error scripting from `tests/common/mod.rs`). Mirrors the
//! RPC-054 pattern in `provider_settings_dispatch_rpc054.rs`. Fully offline:
//! the MockBackend never touches the network or the filesystem.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo, SessionId};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn pinfo(id: &str, configured: bool) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: "api_key".to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

fn profile_def(base_url: &str) -> ProfileDefinition {
    ProfileDefinition {
        base_url: base_url.to_string(),
        api_key: "sk-test".to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
    }
}

/// Drain every pending tokio task spawned by `App::dispatch` AND fold any
/// queued action_tx messages back into the App.
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

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Saving a new openai profile writes and refreshes the list
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_profile_writes_and_refreshes() {
    // @step Given the provider settings view is open with a MockBackend
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", false)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user dispatches SaveProfile for openai profile "work-vllm"
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "work-vllm".to_string(),
        old_profile_name: None,
        definition: profile_def("http://localhost:8888"),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.save_profile_calls() >= 1,
        "save_profile called at least once",
    )
    .await;

    // @step Then backend.save_profile is awaited exactly once
    assert_eq!(mock.save_profile_calls(), 1);

    // @step And the captured save carries provider "openai" and profile "work-vllm"
    let last = mock.last_save_profile().expect("captured save");
    assert_eq!(last.0, "openai");
    assert_eq!(last.1, "work-vllm");

    // @step And the inline status reports the profile was saved
    assert!(
        app.navigator()
            .provider_settings
            .status
            .contains("profile saved"),
        "status should report profile saved, got {:?}",
        app.navigator().provider_settings.status
    );

    // @step And a follow-up backend.list_provider_credentials refresh is dispatched
    assert!(mock.list_provider_credentials_calls() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Editing an existing profile dispatches SaveProfile and repaints
// the row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_profile_dispatches_save_with_new_settings() {
    // @step Given the provider settings view is open with a MockBackend
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", true)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user dispatches SaveProfile for openai profile "home" with baseUrl "http://localhost:9999"
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "home".to_string(),
        old_profile_name: None,
        definition: profile_def("http://localhost:9999"),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.save_profile_calls() >= 1,
        "save_profile called at least once",
    )
    .await;

    // @step Then backend.save_profile is awaited exactly once
    assert_eq!(mock.save_profile_calls(), 1);

    // @step And the captured save definition carries baseUrl "http://localhost:9999"
    let last = mock.last_save_profile().expect("captured save");
    assert_eq!(last.2.base_url, "http://localhost:9999");

    // @step And a follow-up backend.list_provider_credentials refresh is dispatched
    assert!(mock.list_provider_credentials_calls() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Confirming a profile deletion removes it and refreshes the list
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirm_delete_profile_removes_and_refreshes() {
    // @step Given the provider settings view is open with a MockBackend
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", true)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user dispatches ConfirmDeleteProfile for openai profile "work-vllm"
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "work-vllm".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.delete_profile_calls() >= 1,
        "delete_profile called at least once",
    )
    .await;

    // @step Then backend.delete_profile is awaited exactly once
    assert_eq!(mock.delete_profile_calls(), 1);

    // @step And the captured delete carries provider "openai" and profile "work-vllm"
    let last = mock.last_delete_profile().expect("captured delete");
    assert_eq!(last.0, "openai");
    assert_eq!(last.1, "work-vllm");

    // @step And the inline status reports the profile was deleted
    assert!(
        app.navigator()
            .provider_settings
            .status
            .contains("profile deleted"),
        "status should report profile deleted, got {:?}",
        app.navigator().provider_settings.status
    );

    // @step And a follow-up backend.list_provider_credentials refresh is dispatched
    assert!(mock.list_provider_credentials_calls() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A failed profile save surfaces an inline error without leaking
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_save_profile_surfaces_inline_error() {
    // @step Given the provider settings view is open with a MockBackend that fails save_profile with "write failed"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", false)]);
    mock.set_save_profile_error("write failed".to_string());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step When the user dispatches SaveProfile for openai profile "work-vllm"
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "work-vllm".to_string(),
        old_profile_name: None,
        definition: profile_def("http://localhost:8888"),
    });
    drain_pending(&mut app).await;
    wait_until(
        || app.navigator().provider_settings.status.contains('✗'),
        "status reflects the failure",
    )
    .await;

    // @step Then the inline status surfaces the failure with a "✗" marker
    let status = &app.navigator().provider_settings.status;
    assert!(
        status.contains('✗') && status.contains("write failed"),
        "status should surface the failure, got {status:?}"
    );

    // @step And no panic occurs
    // (reaching this line proves the App did not panic)

    // @step And no RPC method name leaks into the agent scrollback
    let ctx = app.agent_view_store().session_context_for(&sid("s-1"));
    if let Some(ctx) = ctx {
        let text = ctx
            .scrollback
            .visible_window(1024)
            .iter()
            .flat_map(|c| {
                c.lines.iter().map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
            })
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            !text.contains("save_profile"),
            "scrollback should not leak RPC method names, got {text:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ConfirmDeleteProfile and DeleteProfile route through the same
// handler
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirm_and_raw_delete_route_through_same_handler() {
    // @step Given the provider settings view is open with a MockBackend
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", true)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user dispatches DeleteProfile for openai profile "a"
    app.dispatch(Action::DeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "a".to_string(),
    });
    drain_pending(&mut app).await;

    // @step And the user dispatches ConfirmDeleteProfile for openai profile "b"
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "b".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.delete_profile_calls() >= 2,
        "delete_profile called twice",
    )
    .await;

    // @step Then backend.delete_profile is awaited exactly twice
    assert_eq!(mock.delete_profile_calls(), 2);

    // @step And both deletes target provider "openai"
    let last = mock.last_delete_profile().expect("captured delete");
    assert_eq!(last.0, "openai");
}
