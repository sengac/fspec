//! RPC-054 — App-level dispatch tests for the ProviderSettingsView.
//!
//! Feature: spec/features/rpc054-provider-settings-dispatch.feature
//!
//! Drives the App::dispatch state machine end-to-end through the new
//! `Action::OpenProviderSettingsView` / `CloseProviderSettingsView` /
//! `SaveProviderCredentials` / `TestProviderConnection` /
//! `RefreshProviderModels` / `DeleteProviderCredentials` flow against
//! the shared `MockBackend` (scripted `list_provider_credentials` +
//! per-call counters from `tests/common/mod.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{
    ModelEntry, ProviderCredentialInfo, SessionId, TestConnectionResult,
};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
    }
}

/// Drain every pending tokio task spawned by `App::dispatch` AND fold
/// any queued action_tx messages back into the App. Mirrors the helper
/// in `pending_input_durability_rpc052.rs`.
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
// Scenario: /provider slash command opens ProviderSettingsView
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_provider_opens_provider_settings_view() {
    // @step Given an App with a MockBackend
    let mock = Arc::new(MockBackend::new());
    // @step And the MockBackend's list_provider_credentials is scripted to return [anthropic api_key configured 8 models, openai api_key not_configured 0 models]
    mock.seed_provider_credentials(vec![
        pinfo("anthropic", "api_key", true, 8),
        pinfo("openai", "api_key", false, 0),
    ]);
    let mut app = fresh_app(mock.clone());

    // @step When the user submits "/provider" via the slash command palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Navigator's active view is ProviderSettings
    assert_eq!(app.navigator().active_view, ViewMode::ProviderSettings);
    // @step And the ProviderSettingsView's provider list contains 2 rows
    assert_eq!(app.navigator().provider_settings.providers.len(), 2);
    // @step And the focused row is "anthropic" with configured indicator "✓" and model count 8
    let focused = app
        .navigator()
        .provider_settings
        .focused_provider()
        .expect("focused row");
    assert_eq!(focused.provider_id, "anthropic");
    assert!(focused.configured);
    assert_eq!(focused.model_count, 8);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Esc returns from ProviderSettingsView to AgentView
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_in_list_mode_returns_to_agent_view() {
    // @step Given the ProviderSettingsView is open in list mode
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    // @step And the previously focused session is s-1
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().active_view, ViewMode::ProviderSettings);

    // @step When the user presses Esc
    app.dispatch(Action::CloseProviderSettingsView);

    // @step Then the Navigator's active view is Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And the current session is s-1
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1"))
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on the API key edit form saves and refreshes the list
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_provider_credentials_writes_and_refreshes() {
    // @step Given the ProviderSettingsView is in edit-api-key mode for "anthropic" with draft "sk-test"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", false, 0)]);
    // Script the list returned after a successful save to reflect the
    // configured indicator flip.
    mock.set_post_save_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user presses Enter
    app.dispatch(Action::SaveProviderCredentials {
        provider_id: "anthropic".to_string(),
        api_key: "sk-test".to_string(),
    });
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || mock.set_provider_credentials_calls() >= 1,
        "set_provider_credentials called at least once",
    )
    .await;

    // @step Then backend.set_provider_credentials is called exactly once with provider_id "anthropic" and an ApiKey input with key "sk-test"
    assert_eq!(mock.set_provider_credentials_calls(), 1);
    let last = mock.last_set_provider_credentials().expect("captured");
    assert_eq!(last.0, "anthropic");
    assert_eq!(last.1.kind, "api_key");
    assert_eq!(last.1.api_key.as_deref(), Some("sk-test"));

    // @step And backend.list_provider_credentials is called at least once after the save
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the ProviderSettingsView is back in list mode
    use codelet_fspec_tui::views::ProviderSettingsMode;
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    ));

    // @step And the anthropic row shows configured indicator "✓"
    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "anthropic")
        .expect("anthropic row present");
    assert!(row.configured);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 't' on a row runs a connection test
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_provider_connection_success_renders_ok_latency() {
    // @step Given the ProviderSettingsView is open with the openai row focused
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", "api_key", true, 4)]);
    // @step And the MockBackend's test_provider_connection is scripted to return TestConnectionResult{ success: true, error: None, latency_ms: 42 } for "openai"
    mock.set_test_connection_result(
        "openai",
        TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 42,
        },
    );
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user presses "t"
    app.dispatch(Action::TestProviderConnection("openai".to_string()));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || mock.test_provider_connection_calls() >= 1,
        "test_provider_connection called at least once",
    )
    .await;

    // @step Then backend.test_provider_connection is called exactly once with "openai"
    assert_eq!(mock.test_provider_connection_calls(), 1);
    assert_eq!(mock.last_test_provider_connection().as_deref(), Some("openai"));

    // @step And the right-pane status area shows "✓ ok (42ms)"
    let status = &app.navigator().provider_settings.status;
    assert!(
        status.contains("✓") && status.contains("ok") && status.contains("42"),
        "status should mention ok + 42ms, got {status:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 't' surfaces backend errors inline
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_provider_connection_error_renders_inline() {
    // @step Given the ProviderSettingsView is open with the openai row focused
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("openai", "api_key", true, 4)]);
    // @step And the MockBackend's test_provider_connection is scripted to return TestConnectionResult{ success: false, error: Some("unreachable: dns resolution failed"), latency_ms: 0 } for "openai"
    mock.set_test_connection_result(
        "openai",
        TestConnectionResult {
            success: false,
            error: Some("unreachable: dns resolution failed".to_string()),
            latency_ms: 0,
        },
    );
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user presses "t"
    app.dispatch(Action::TestProviderConnection("openai".to_string()));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || app.navigator().provider_settings.status.contains("✗"),
        "status reflects error",
    )
    .await;

    // @step Then the right-pane status area contains "✗ unreachable: dns resolution failed"
    let status = &app.navigator().provider_settings.status;
    assert!(
        status.contains("unreachable: dns resolution failed"),
        "status should contain the scripted error, got {status:?}"
    );

    // @step And the openai row's configured indicator is unchanged
    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "openai")
        .expect("openai row present");
    assert!(row.configured, "configured flag should be unchanged");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'r' refreshes the model list and updates the row count
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_models_updates_row_count() {
    // @step Given the ProviderSettingsView is open with the openai row focused
    let mock = Arc::new(MockBackend::new());
    // @step And the openai row's model count is 4
    mock.seed_provider_credentials(vec![pinfo("openai", "api_key", true, 4)]);
    // @step And the MockBackend's refresh_models_cache is scripted to return a 8-entry model list for "openai"
    let models: Vec<ModelEntry> = (0..8)
        .map(|i| ModelEntry {
            id: format!("m{i}"),
            display_name: format!("Model {i}"),
            context_window: 8192,
            supports_reasoning: false,
            supports_vision: false,
            is_custom: false,
        })
        .collect();
    mock.set_refresh_models_result("openai", models);
    // @step And the MockBackend's list_provider_credentials is scripted to return [openai api_key configured 8 models] after the refresh
    mock.set_post_refresh_provider_credentials(vec![pinfo("openai", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user presses "r"
    app.dispatch(Action::RefreshProviderModels("openai".to_string()));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || mock.refresh_models_cache_calls() >= 1,
        "refresh_models_cache called at least once",
    )
    .await;

    // @step Then backend.refresh_models_cache is called exactly once with "openai"
    assert_eq!(mock.refresh_models_cache_calls(), 1);
    assert_eq!(mock.last_refresh_models_cache().as_deref(), Some("openai"));

    // @step And backend.list_provider_credentials is called at least once after the refresh
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the openai row's model count is 8
    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "openai")
        .expect("openai row present");
    assert_eq!(row.model_count, 8);

    // @step And the right-pane status area contains "models refreshed"
    assert!(
        app.navigator()
            .provider_settings
            .status
            .contains("models refreshed"),
        "status should mention models refreshed, got {:?}",
        app.navigator().provider_settings.status
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'd' on a configured row clears the credentials
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_provider_credentials_clears_row() {
    // @step Given the ProviderSettingsView is open with the anthropic row focused
    // @step And the anthropic row is configured
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    // @step And the MockBackend's list_provider_credentials is scripted to return [anthropic api_key not_configured 0 models] after the delete
    mock.set_post_delete_provider_credentials(vec![pinfo("anthropic", "api_key", false, 0)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user presses "d"
    app.dispatch(Action::DeleteProviderCredentials("anthropic".to_string()));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || mock.delete_provider_credentials_calls() >= 1,
        "delete_provider_credentials called at least once",
    )
    .await;

    // @step Then backend.delete_provider_credentials is called exactly once with "anthropic"
    assert_eq!(mock.delete_provider_credentials_calls(), 1);
    assert_eq!(
        mock.last_delete_provider_credentials().as_deref(),
        Some("anthropic")
    );

    // @step And backend.list_provider_credentials is called at least once after the delete
    assert!(mock.list_provider_credentials_calls() >= 1);

    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "anthropic")
        .expect("anthropic row present");
    // @step And the anthropic row shows configured indicator "(not configured)"
    assert!(!row.configured);
    // @step And the anthropic row's model count is 0
    assert_eq!(row.model_count, 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Backend errors are silently logged without panicking
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn backend_errors_are_silently_logged_without_panicking() {
    // @step Given the ProviderSettingsView is open with the anthropic row focused
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", false, 0)]);
    // @step And the MockBackend's set_provider_credentials is scripted to return Err("write failed")
    mock.set_set_provider_credentials_error("write failed".to_string());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user opens the API-key edit form for anthropic, types "sk-test", and presses Enter
    app.dispatch(Action::SaveProviderCredentials {
        provider_id: "anthropic".to_string(),
        api_key: "sk-test".to_string(),
    });
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || mock.set_provider_credentials_calls() >= 1,
        "set_provider_credentials attempted",
    )
    .await;

    // @step Then the App must not panic
    // (reaching this line proves the App did not panic)

    // @step And no scrollback chunks contain the text "set_provider_credentials"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"));
    if let Some(ctx) = ctx {
        let text = ctx
            .scrollback
            .visible_window(1024)
            .iter()
            .flat_map(|c| {
                c.lines
                    .iter()
                    .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            })
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            !text.contains("set_provider_credentials"),
            "scrollback should not leak RPC method names, got {text:?}"
        );
    }

    // @step And the right-pane status area contains "✗ write failed"
    let status = &app.navigator().provider_settings.status;
    assert!(
        status.contains("✗") && status.contains("write failed"),
        "status should surface the failure, got {status:?}"
    );
}
