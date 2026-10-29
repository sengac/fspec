//! Feature: spec/features/rpc054-provider-settings-dispatch.feature

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_provider_credentials_writes_and_refreshes() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-test-1"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", false, 0)]);
    mock.set_post_save_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());

    // @step When the user presses Enter
    app.dispatch(Action::SaveProviderCredentials {
        provider_id: "anthropic".to_string(),
        api_key: "sk-test-1".to_string(),
    });
    drain_pending(&mut app).await;
    wait_until(
        || mock.set_provider_credentials_calls() >= 1,
        "set_provider_credentials called at least once",
    )
    .await;

    // @step Then Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-test-1" } is dispatched
    // (proven by the round-trip below)
    // @step And backend.set_provider_credentials("anthropic", ProviderCredentialInput::api_key("sk-test-1")) is awaited
    assert_eq!(mock.set_provider_credentials_calls(), 1);
    let last = mock.last_set_provider_credentials().expect("captured");
    assert_eq!(last.0, "anthropic");
    assert_eq!(last.1.kind, "api_key");
    assert_eq!(last.1.api_key.as_deref(), Some("sk-test-1"));

    // @step And on Ok the action Action::ProviderSettingsStatus("✓ anthropic credentials saved") is dispatched
    assert!(
        app.navigator()
            .provider_settings
            .status
            .contains("credentials saved"),
        "status should report credentials saved, got {:?}",
        app.navigator().provider_settings.status
    );
    // @step And a follow-up backend.list_provider_credentials() refresh is dispatched
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the resulting Action::ProviderCredentialsLoaded folds the new list into the view
    use codelet_fspec_tui::views::ProviderSettingsMode;
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    ));
    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "anthropic")
        .expect("anthropic row present");
    assert!(row.configured);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_provider_credentials_clears_row() {
    // @step Given the ProviderSettingsView's ConfirmDialog is open for "anthropic" with Primary focused
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    mock.set_post_delete_provider_credentials(vec![pinfo("anthropic", "api_key", false, 0)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step When the user presses Enter
    app.dispatch(Action::ConfirmDeleteProviderCredentials(
        "anthropic".to_string(),
    ));
    drain_pending(&mut app).await;
    wait_until(
        || mock.delete_provider_credentials_calls() >= 1,
        "delete_provider_credentials called at least once",
    )
    .await;

    // @step Then Action::ConfirmDeleteProviderCredentials("anthropic") is dispatched
    // @step And backend.delete_provider_credentials("anthropic") is awaited
    assert_eq!(mock.delete_provider_credentials_calls(), 1);
    assert_eq!(
        mock.last_delete_provider_credentials().as_deref(),
        Some("anthropic")
    );

    // @step And on Ok the action Action::ProviderSettingsStatus("✓ anthropic credentials cleared") is dispatched
    assert!(
        app.navigator()
            .provider_settings
            .status
            .contains("credentials cleared")
            || app.navigator().provider_settings.status.contains("✓"),
        "status should report credentials cleared, got {:?}",
        app.navigator().provider_settings.status
    );
    // @step And a follow-up backend.list_provider_credentials() refresh is dispatched
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the anthropic row repaints with configured = false and model_count = 0
    let row = app
        .navigator()
        .provider_settings
        .providers
        .iter()
        .find(|p| p.provider_id == "anthropic")
        .expect("anthropic row present");
    assert!(!row.configured);
    assert_eq!(row.model_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d_on_configured_row_opens_confirm_dialog_before_backend() {
    use codelet_fspec_tui::views::agent::confirm_dialog::ConfirmDialog;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused (configured = true)
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator()
            .provider_settings
            .focused_provider()
            .map(|p| p.provider_id.as_str()),
        Some("anthropic")
    );

    // @step When the user presses "d"
    let _ = app
        .navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the ConfirmDialog is mounted
    let dialog: &Option<ConfirmDialog> = &app.navigator().provider_settings.delete_confirm;
    assert!(dialog.is_some(), "ConfirmDialog should be open");

    // @step And NO Action::DeleteProviderCredentials nor Action::ConfirmDeleteProviderCredentials is dispatched
    // @step And backend.delete_provider_credentials is NEVER called
    assert_eq!(mock.delete_provider_credentials_calls(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_confirm_dialog_cancels_without_backend_round_trip() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // @step Given the ProviderSettingsView's ConfirmDialog is open for "anthropic"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;
    let _ = app
        .navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert!(app.navigator().provider_settings.delete_confirm.is_some());

    // @step When the user presses Esc
    let _ = app
        .navigator_mut()
        .provider_settings
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the ConfirmDialog is dismissed
    assert!(app.navigator().provider_settings.delete_confirm.is_none());

    // @step And NO Action::ConfirmDeleteProviderCredentials is dispatched
    // @step And backend.delete_provider_credentials is NEVER called
    assert_eq!(mock.delete_provider_credentials_calls(), 0);
}
