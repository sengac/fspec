//! PROV-112 section 1: ProviderSettingsView key handling for the disconnect confirm flow.

use super::*;
use codelet_test_helpers::TestResult;

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
fn enter_on_oauth_status_opens_disconnect_oauth_confirm() -> TestResult {
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
        other => return Err(format!("expected DisconnectOAuth mode, got {other:?}").into()),
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing d on an oauth-status row opens the DisconnectOAuth
// confirm not the api-key delete
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_oauth_status_opens_disconnect_oauth_confirm_not_api_key_delete() -> TestResult {
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
        other => return Err(format!("expected DisconnectOAuth mode, got {other:?}").into()),
    }

    // @step And the generic delete-credentials confirm is not opened
    assert!(
        view.delete_confirm.is_none(),
        "`d` on oauth-status must NOT open the generic delete-credentials confirm"
    );
    assert!(matches!(event, ProviderSettingsEvent::Consumed));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cancelling disconnect with Esc preserves the tokens
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn esc_in_disconnect_confirm_preserves_tokens_and_returns_to_list() -> TestResult {
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cancelling disconnect with n makes no backend call
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn n_in_disconnect_confirm_makes_no_backend_call() -> TestResult {
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An unrelated key in the confirm dialog is consumed and the
// dialog stays open
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn unrelated_key_in_disconnect_confirm_is_consumed_and_dialog_stays_open() -> TestResult {
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
        other => {
            return Err(format!("confirm must stay open as DisconnectOAuth, got {other:?}").into())
        }
    }

    // @step And no backend clear call is made
    // (the consumed event carries no Action — asserted above)
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: Disconnect routes to the correct per-provider clear
// method — the VIEW half asserts `y` emits OAuthDisconnect keyed by the
// focused provider for each built-in. (The backend per-provider clear-method
// routing is asserted at the dispatch layer.)
// ─────────────────────────────────────────────────────────────────────────

fn assert_y_emits_disconnect_for(
    provider: &str,
    name: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    // @step Given a provider "<provider>" is expanded with OAuth tokens present
    let mut view = view_on_logout_row(provider, name);
    // @step And the user has opened the DisconnectOAuth confirm
    view.handle_key(key(KeyCode::Enter));

    // @step When the user presses "y"
    let event = view.handle_key(char_key('y'));

    // @step Then the backend "<clear_method>" is called exactly once for provider "<provider>"
    // (view layer: `y` emits exactly one OAuthDisconnect keyed by this provider)
    match event {
        ProviderSettingsEvent::Emit(action) => {
            let Action::OAuthDisconnect { provider_id } = *action else {
                return Err(format!(
                    "expected Emit(OAuthDisconnect) for {provider}, got {action:?}"
                )
                .into());
            };
            assert_eq!(provider_id, provider);
        }
        other => {
            return Err(
                format!("expected Emit(OAuthDisconnect) for {provider}, got {other:?}").into(),
            )
        }
    }
    // And the confirm closes back to the list.
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    Ok(())
}

#[test]
fn y_emits_disconnect_keyed_by_provider_for_each_builtin() -> TestResult {
    assert_y_emits_disconnect_for("anthropic", "Claude")?;
    assert_y_emits_disconnect_for("codex", "ChatGPT")?;
    assert_y_emits_disconnect_for("github-copilot", "GitHub Copilot")?;
    Ok(())
}
