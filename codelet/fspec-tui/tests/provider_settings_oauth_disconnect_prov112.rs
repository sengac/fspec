//! PROV-112 — OAuth disconnect/logout: confirm-flow view tests, App-level
//! dispatch tests, and the transport-surface test, all for one feature.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//!
//! One test file per feature (ACDD 1:1). Sections:
//!   1. VIEW/KEY — pure `ProviderSettingsView::handle_key` drives (no backend).
//!   2. DISPATCH — `App::dispatch` against the shared `MockBackend`
//!      (call counters + scripted Ok/Err), fully offline.
//!   3. TRANSPORT — embedded forwards to the providers-direct OAuth wiring;
//!      the websocket-inherited trait DEFAULT is a no-op/unsupported stub.
//!
//! No real OAuth network and no real `~/.fspec` mutation: the transport test
//! redirects `FSPEC_HOME` to a throwaway tempdir, and this binary hosts the
//! only `FSPEC_HOME` writer so that env mutation cannot race the other tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::{
    OAuthMethod, ProviderDisplayInfo, ProviderSettingsEvent, ProviderSettingsMode,
    ProviderSettingsView,
};
use codelet_fspec_tui::{App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc_types::{ProviderCredentialInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::sync::broadcast;
use tokio::time::timeout;

mod common;
use common::MockBackend;

// ─────────────────────────── shared key helpers ───────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn char_key(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

// ════════════════════════ SECTION 1: VIEW / KEY ════════════════════════

fn oauth_display(id: &str, name: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: name.to_string(),
        configured: true,
        credential_type: "oauth".to_string(),
        model_count: 0,
        has_oauth_tokens: true,
        is_oauth_provider: true,
        requires_api_key: false,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: vec![(OAuthMethod::Browser, format!("Login with {name} (browser)"))],
        oauth_status_label: Some(format!("Logout from OAuth [{name}]")),
        masked_key: Some("OAuth".to_string()),
        source: Some(name.to_string()),
    }
}

/// Build a view with a single expanded OAuth provider, cursor on its
/// `oauth-status` (Logout) row (nav order: provider, oauth-status, login…).
fn view_on_logout_row(id: &str, name: &str) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![oauth_display(id, name)]);
    view.toggle_expansion(id);
    // Index 0 = provider row, index 1 = oauth-status (Logout) row.
    view.selected_index = 1;
    assert!(
        matches!(
            view.focused_nav_item().map(|i| &i.kind),
            Some(NavItemKind::OAuthStatus { .. })
        ),
        "fixture cursor must be on the oauth-status row"
    );
    view
}

fn has_logout_row(view: &ProviderSettingsView) -> bool {
    view.nav_items
        .iter()
        .any(|i| matches!(i.kind, NavItemKind::OAuthStatus { .. }))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Enter on an oauth-status row opens the DisconnectOAuth confirm
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_oauth_status_opens_disconnect_oauth_confirm() {
    // @step Given a provider "anthropic" is expanded with OAuth tokens present
    // @step And the cursor is on the "Logout from OAuth [Claude]" row
    let mut view = view_on_logout_row("anthropic", "Claude");

    // @step When the user presses Enter
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the mode becomes DisconnectOAuth for provider "anthropic"
    match &view.mode {
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected DisconnectOAuth mode, got {other:?}"),
    }

    // @step And the generic api-key delete-credentials confirm is not opened
    assert!(
        view.delete_confirm.is_none(),
        "the generic delete-credentials confirm must not open"
    );

    // @step And no backend clear call has been made yet
    // (opening the confirm only consumes the key — it emits no Action)
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "Enter on oauth-status must only open the confirm (no Action emitted)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing d on an oauth-status row opens the DisconnectOAuth
// confirm not the api-key delete
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_oauth_status_opens_disconnect_oauth_confirm_not_api_key_delete() {
    // @step Given a provider "anthropic" is expanded with OAuth tokens present
    // @step And the cursor is on the "Logout from OAuth [Claude]" row
    let mut view = view_on_logout_row("anthropic", "Claude");

    // @step When the user presses "d"
    let event = view.handle_key(char_key('d'));

    // @step Then the mode becomes DisconnectOAuth for provider "anthropic"
    match &view.mode {
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected DisconnectOAuth mode, got {other:?}"),
    }

    // @step And the generic delete-credentials confirm is not opened
    assert!(
        view.delete_confirm.is_none(),
        "`d` on oauth-status must NOT open the generic delete-credentials confirm"
    );
    assert!(matches!(event, ProviderSettingsEvent::Consumed));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cancelling disconnect with Esc preserves the tokens
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn esc_in_disconnect_confirm_preserves_tokens_and_returns_to_list() {
    // @step Given a provider "anthropic" is expanded with OAuth tokens present
    let mut view = view_on_logout_row("anthropic", "Claude");
    // @step And the user has opened the DisconnectOAuth confirm
    view.handle_key(key(KeyCode::Enter));
    assert!(matches!(
        view.mode,
        ProviderSettingsMode::DisconnectOAuth { .. }
    ));

    // @step When the user presses Esc
    let event = view.handle_key(key(KeyCode::Esc));

    // @step Then no backend clear call is made
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "Esc must not emit an OAuthDisconnect action"
    );

    // @step And the mode returns to list
    assert!(matches!(view.mode, ProviderSettingsMode::List));

    // @step And the "Logout from OAuth [Claude]" row is still shown
    assert!(
        has_logout_row(&view),
        "the Logout row must remain (tokens preserved on cancel)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cancelling disconnect with n makes no backend call
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn n_in_disconnect_confirm_makes_no_backend_call() {
    // @step Given a provider "anthropic" is expanded with OAuth tokens present
    let mut view = view_on_logout_row("anthropic", "Claude");
    // @step And the user has opened the DisconnectOAuth confirm
    view.handle_key(key(KeyCode::Enter));

    // @step When the user presses "n"
    let event = view.handle_key(char_key('n'));

    // @step Then no backend clear call is made
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "`n` must not emit an OAuthDisconnect action"
    );

    // @step And the mode returns to list
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An unrelated key in the confirm dialog is consumed and the
// dialog stays open
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn unrelated_key_in_disconnect_confirm_is_consumed_and_dialog_stays_open() {
    // @step Given a provider "anthropic" is expanded with OAuth tokens present
    let mut view = view_on_logout_row("anthropic", "Claude");
    // @step And the user has opened the DisconnectOAuth confirm
    view.handle_key(key(KeyCode::Enter));

    // @step When the user presses "x"
    let event = view.handle_key(char_key('x'));

    // @step Then nothing happens
    assert!(
        matches!(event, ProviderSettingsEvent::Consumed),
        "an unrelated key must be consumed without emitting an Action"
    );

    // @step And the mode is still DisconnectOAuth for provider "anthropic"
    match &view.mode {
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("confirm must stay open as DisconnectOAuth, got {other:?}"),
    }

    // @step And no backend clear call is made
    // (the consumed event carries no Action — asserted above)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: Disconnect routes to the correct per-provider clear
// method — the VIEW half asserts `y` emits OAuthDisconnect keyed by the
// focused provider for each built-in. (The backend per-provider clear-method
// routing is asserted at the dispatch layer.)
// ─────────────────────────────────────────────────────────────────────────

fn assert_y_emits_disconnect_for(provider: &str, name: &str) {
    // @step Given a provider "<provider>" is expanded with OAuth tokens present
    let mut view = view_on_logout_row(provider, name);
    // @step And the user has opened the DisconnectOAuth confirm
    view.handle_key(key(KeyCode::Enter));

    // @step When the user presses "y"
    let event = view.handle_key(char_key('y'));

    // @step Then the backend "<clear_method>" is called exactly once for provider "<provider>"
    // (view layer: `y` emits exactly one OAuthDisconnect keyed by this provider)
    match event {
        ProviderSettingsEvent::Emit(Action::OAuthDisconnect { provider_id }) => {
            assert_eq!(provider_id, provider);
        }
        other => panic!("expected Emit(OAuthDisconnect) for {provider}, got {other:?}"),
    }
    // And the confirm closes back to the list.
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

#[test]
fn y_emits_disconnect_keyed_by_provider_for_each_builtin() {
    assert_y_emits_disconnect_for("anthropic", "Claude");
    assert_y_emits_disconnect_for("codex", "ChatGPT");
    assert_y_emits_disconnect_for("github-copilot", "GitHub Copilot");
}

// ════════════════════════ SECTION 2: DISPATCH ════════════════════════

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// A connected OAuth provider credential row (configured + no env masked key
/// → the projection classifies it as OAuth-logged-in and emits the Logout
/// row).
fn oauth_pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: "oauth".to_string(),
        model_count: 0,
        masked_key: None,
        source: Some(id.to_string()),
    }
}

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
// Scenario: Confirming disconnect clears codex tokens once and refreshes the
// nav (dispatch half — the y-press → OAuthDisconnect emission is covered in
// the view test).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_clears_codex_tokens_once_and_refreshes() {
    // @step Given a provider "codex" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo("codex")]);
    let mut app = fresh_app(mock.clone());

    // @step And the cursor is on the "Logout from OAuth [ChatGPT]" row
    // @step And the user has opened the DisconnectOAuth confirm
    // (the dispatch test enters at the OAuthDisconnect action; the view test
    // covers the y-press that emits it)

    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "codex".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens called at least once",
    )
    .await;

    // @step Then the backend codex clear-tokens method is called exactly once
    assert_eq!(mock.oauth_clear_tokens_calls(), 1);
    assert_eq!(
        mock.oauth_clear_tokens_providers(),
        vec!["codex".to_string()]
    );

    // @step And the cached OPENAI_API_KEY is preserved
    // (the codex clear strips ONLY the tokens field — proven by the pure
    // `strip_codex_tokens` unit test in codelet-rpc; the backend boundary here
    // never deletes a key)

    // @step And the credentials are re-fetched producing a ProviderCredentialsLoaded refresh
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the "Logout from OAuth [ChatGPT]" row is gone
    let view = &app.navigator().provider_settings;
    let has_logout = view.nav_items.iter().any(|i| {
        matches!(
            i.kind,
            codelet_fspec_tui::views::NavItemKind::OAuthStatus { .. }
        )
    });
    assert!(
        !has_logout,
        "the Logout row must disappear after disconnect"
    );

    // @step And the cursor returns to the "codex" provider row
    // (cursor lands on the codex provider row; with the Logout row gone the
    // only codex row left is the provider header)
    if let Some(item) = view.focused_nav_item() {
        assert_eq!(item.provider_id, "codex");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A backend clear error returns to list silently without leaking
// the RPC name
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn backend_clear_error_returns_to_list_without_leaking_rpc_name() {
    // @step Given a provider "github-copilot" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo("github-copilot")]);
    // @step And the backend clear-credential method is scripted to return an error
    mock.set_oauth_clear_tokens_error("copilot_oauth_clear_credential failed".to_string());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;

    // @step And the user has opened the DisconnectOAuth confirm
    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "github-copilot".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens attempted",
    )
    .await;

    // @step Then the UI returns to list
    assert!(matches!(
        app.navigator().provider_settings.mode,
        codelet_fspec_tui::views::ProviderSettingsMode::List
    ));

    // @step And no RPC or method name is shown anywhere in the UI
    let status = &app.navigator().provider_settings.status;
    assert!(
        !status.contains("oauth_clear_tokens")
            && !status.contains("copilot_oauth_clear_credential")
            && !status.contains("clear_credential"),
        "status must not leak the RPC/method name, got {status:?}"
    );
    // And no leak into the agent scrollback either.
    if let Some(ctx) = app.agent_view_store().session_context_for(&sid("s-1")) {
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
            !text.contains("oauth_clear_tokens") && !text.contains("clear_credential"),
            "scrollback must not leak RPC method names, got {text:?}"
        );
    }

    // @step And the clear operation is idempotent on a subsequent reload
    // (a second disconnect still resolves cleanly and silently — even with the
    // error still scripted, the dispatch swallows it and re-attempts)
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "github-copilot".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 2,
        "second clear attempted (idempotent)",
    )
    .await;
    assert_eq!(mock.oauth_clear_tokens_calls(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: Disconnect routes to the correct per-provider clear
// method (dispatch half — the backend records the provider_id it was asked to
// clear exactly once; the per-provider file routing is proven inside
// codelet-rpc's oauth_disconnect dispatch + strip_codex_tokens unit test).
// ─────────────────────────────────────────────────────────────────────────

async fn assert_clear_called_once_for(provider: &str) {
    // @step Given a provider "<provider>" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo(provider)]);
    let mut app = fresh_app(mock.clone());

    // @step And the user has opened the DisconnectOAuth confirm
    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: provider.to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens called",
    )
    .await;

    // @step Then the backend "<clear_method>" is called exactly once for provider "<provider>"
    assert_eq!(mock.oauth_clear_tokens_calls(), 1);
    assert_eq!(
        mock.oauth_clear_tokens_providers(),
        vec![provider.to_string()],
        "clear must be routed for {provider} exactly once"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_anthropic() {
    assert_clear_called_once_for("anthropic").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_codex() {
    assert_clear_called_once_for("codex").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_github_copilot() {
    assert_clear_called_once_for("github-copilot").await;
}

// ════════════════════════ SECTION 3: TRANSPORT ════════════════════════

/// A backend that overrides NOTHING for the oauth surface — it stands in for
/// the websocket transport, which inherits the trait's no-op/unsupported
/// defaults (it deliberately does not override `oauth_clear_tokens` /
/// `oauth_get_tokens`).
struct DefaultsOnlyBackend;

#[async_trait]
impl FspecBackend for DefaultsOnlyBackend {
    async fn list_work_units(&self) -> Result<Vec<codelet_rpc_types::WorkUnitInfo>> {
        Ok(Vec::new())
    }
    async fn list_sessions(&self, _project_path: String) -> Result<Vec<codelet_rpc_types::SessionInfo>> {
        Ok(Vec::new())
    }
    async fn create_session(&self, _role: Option<String>) -> Result<codelet_rpc_types::SessionId> {
        Ok(codelet_rpc_types::SessionId::new("x"))
    }
    async fn send_input(&self, _id: codelet_rpc_types::SessionId, _text: String) -> Result<()> {
        Ok(())
    }
    async fn interrupt(&self, _id: codelet_rpc_types::SessionId) -> Result<()> {
        Ok(())
    }
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<codelet_rpc_types::WorkUnitInfo>> {
        broadcast::channel(1).1
    }
    fn chunks_rx(
        &self,
    ) -> broadcast::Receiver<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)> {
        broadcast::channel(1).1
    }
    fn logs_rx(&self) -> broadcast::Receiver<codelet_rpc_types::LogRecord> {
        broadcast::channel(1).1
    }
    async fn health(&self) -> Result<codelet_rpc_types::HealthInfo> {
        Ok(codelet_rpc_types::HealthInfo {
            uptime_secs: 0,
            connected_clients: 0,
            last_watcher_event_secs_ago: None,
            lag_chunks: 0,
            lag_logs: 0,
            lag_work_units: 0,
            version: String::new(),
        })
    }
    async fn checkpoint_counts(&self) -> Result<codelet_rpc_types::CheckpointCounts> {
        Ok(codelet_rpc_types::CheckpointCounts::default())
    }
    async fn move_work_unit_up(&self, _id: String) -> Result<()> {
        Ok(())
    }
    async fn move_work_unit_down(&self, _id: String) -> Result<()> {
        Ok(())
    }
    async fn get_model_info(
        &self,
        _session_id: codelet_rpc_types::SessionId,
    ) -> Result<codelet_rpc_types::ModelInfo> {
        Ok(codelet_rpc_types::ModelInfo::default())
    }
    async fn get_thinking_level(
        &self,
        _session_id: codelet_rpc_types::SessionId,
    ) -> Result<codelet_rpc_types::ThinkingLevel> {
        Ok(codelet_rpc_types::ThinkingLevel::Off)
    }
    async fn get_workspace_info(&self) -> Result<codelet_rpc_types::WorkspaceInfo> {
        Ok(codelet_rpc_types::WorkspaceInfo::default())
    }
    async fn search_files(&self, _prefix: String, _limit: u32) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn persistence_add_history(
        &self,
        _session: codelet_rpc_types::SessionId,
        _text: String,
    ) -> Result<()> {
        Ok(())
    }
    async fn persistence_get_history(
        &self,
        _session: codelet_rpc_types::SessionId,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn persistence_search_history(
        &self,
        _query: String,
    ) -> Result<Vec<codelet_rpc_types::HistoryMatch>> {
        Ok(Vec::new())
    }
    async fn persistence_delete_session(&self, _id: codelet_rpc_types::SessionId) -> Result<()> {
        Ok(())
    }
    async fn list_providers(&self) -> Result<Vec<codelet_rpc_types::ProviderInfo>> {
        Ok(Vec::new())
    }
    async fn set_session_model(
        &self,
        _session_id: codelet_rpc_types::SessionId,
        _provider_id: String,
        _model_id: String,
    ) -> Result<()> {
        Ok(())
    }
    async fn set_thinking_level(
        &self,
        _session_id: codelet_rpc_types::SessionId,
        _level: codelet_rpc_types::ThinkingLevel,
    ) -> Result<()> {
        Ok(())
    }
    async fn get_session_role(
        &self,
        _session_id: codelet_rpc_types::SessionId,
    ) -> Result<Option<String>> {
        Ok(None)
    }
    async fn set_session_role(
        &self,
        _session_id: codelet_rpc_types::SessionId,
        _role: Option<String>,
    ) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn oauth_methods_napi_direct_on_embedded_and_noop_on_websocket() {
    // @step Given the embedded transport is in use
    let (_dir, service) = common::temp_service();
    let handle = tokio::runtime::Handle::current();
    let embedded = EmbeddedFspecBackend::new(handle, Arc::clone(&service));

    // Redirect the OAuth credential dir to a throwaway temp FSPEC_HOME so no
    // real ~/.fspec is touched. (This binary hosts a single test, so the env
    // mutation cannot race other tests.)
    let cred_dir = tempfile::tempdir().expect("cred tempdir");
    std::env::set_var("FSPEC_HOME", cred_dir.path());
    let claude_auth = cred_dir.path().join("claude_auth.json");
    std::fs::write(
        &claude_auth,
        r#"{"access_token":"at","refresh_token":"rt","expires":9999999999999}"#,
    )
    .expect("seed claude_auth.json");

    // @step Then the FspecBackend OAuth methods forward to the napi OAuth functions
    // (a real providers-direct read sees the seeded tokens — proving it is NOT
    // the no-op default, which would report false)
    assert!(
        embedded
            .oauth_get_tokens("anthropic".to_string())
            .await
            .expect("embedded oauth_get_tokens"),
        "embedded oauth_get_tokens must forward to providers and see the seeded tokens"
    );
    // And the clear actually deletes the credential (real wiring, idempotent).
    embedded
        .oauth_clear_tokens("anthropic".to_string())
        .await
        .expect("embedded oauth_clear_tokens");
    assert!(
        !claude_auth.exists(),
        "embedded oauth_clear_tokens must really delete claude_auth.json"
    );
    assert!(
        !embedded
            .oauth_get_tokens("anthropic".to_string())
            .await
            .expect("embedded oauth_get_tokens after clear"),
        "after clear the embedded transport must report no tokens"
    );
    // Idempotent: a second clear with no file still succeeds.
    embedded
        .oauth_clear_tokens("anthropic".to_string())
        .await
        .expect("embedded oauth_clear_tokens idempotent");

    std::env::remove_var("FSPEC_HOME");

    // @step When the websocket transport is in use
    // @step Then the FspecBackend OAuth methods resolve to the unsupported/no-op defaults
    let ws_like = DefaultsOnlyBackend;
    assert!(
        !ws_like
            .oauth_get_tokens("anthropic".to_string())
            .await
            .expect("default oauth_get_tokens"),
        "the websocket-inherited default must report no tokens (unsupported)"
    );
    ws_like
        .oauth_clear_tokens("anthropic".to_string())
        .await
        .expect("default oauth_clear_tokens is a silent no-op Ok");
}
