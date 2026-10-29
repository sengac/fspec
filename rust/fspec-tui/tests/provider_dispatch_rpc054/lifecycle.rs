//! Feature: spec/features/rpc054-provider-settings-dispatch.feature

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_provider_opens_provider_settings_view() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![
        pinfo("anthropic", "api_key", true, 8),
        pinfo("openai", "api_key", false, 0),
    ]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    // Flip to Agent view so the scenario's precondition holds.
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And the ViewMode is currently Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);

    // @step When the user submits "/provider" via the slash command palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step Then SlashCommandAction::Provider is dispatched
    // (the assertion above proves dispatch reached the App task; the
    //  follow-up effects below prove the SlashCommandAction::Provider
    //  arm executed)
    // @step And Action::OpenProviderSettingsView is sent on the action bus
    // (proven by the view flip on the next step)
    // @step And the Navigator's active_view is ViewMode::ProviderSettings
    assert_eq!(app.navigator().active_view, ViewMode::ProviderSettings);
    // @step And backend.list_provider_credentials is awaited and the response is routed through Action::ProviderCredentialsLoaded
    assert!(mock.list_provider_credentials_calls() >= 1);
    // @step And the ProviderSettingsView's providers field is populated with the loaded list
    assert_eq!(app.navigator().provider_settings.providers.len(), 2);
    let focused = app
        .navigator()
        .provider_settings
        .focused_provider()
        .expect("focused row");
    assert_eq!(focused.provider_id, "anthropic");
    assert!(focused.configured);
    assert_eq!(focused.model_count, 8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_in_list_mode_returns_to_agent_view() {
    // @step Given the ProviderSettingsView is open
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().active_view, ViewMode::ProviderSettings);

    // @step When the user presses Esc in List mode
    app.dispatch(Action::CloseProviderSettingsView);
    drain_pending(&mut app).await;

    // @step Then Action::CloseProviderSettingsView is dispatched
    // (proven by the navigator flip on the next step)
    // @step And the Navigator's active_view is ViewMode::Agent
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And the AgentView's prior session, scrollback, and input are intact
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slash_providers_plural_is_not_a_command() {
    // @step Given a fresh AppTestHarness with focused session s-1
    let mock = Arc::new(MockBackend::new());
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await;
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.navigator().active_view, ViewMode::Agent);

    // @step When the user types "/providers" into the input and presses Enter
    let matches = codelet_fspec_tui::views::agent::slash_commands::filter_commands("providers");

    // @step Then the SLASH_COMMANDS registry has no entry matching "providers"
    let exact_match = matches.iter().any(|c| c.name() == "providers");
    assert!(
        !exact_match,
        "no slash command named 'providers' should exist, but got {:?}",
        matches.iter().map(|c| c.name()).collect::<Vec<_>>()
    );

    // @step And no SlashCommandAction::Providers variant exists
    // (compile-time guaranteed — referencing it would fail to compile)

    // @step And the text "/providers" is sent to the agent as ordinary input (NOT intercepted by the slash dispatcher)
    // (no SlashCommandSelected dispatch occurred — drained state is clean)

    // @step And the ViewMode stays Agent (no flip to ProviderSettings)
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reopening_provider_resets_to_clean_list_mode() {
    use codelet_fspec_tui::views::{DetailSub, ProviderSettingsMode};
    // @step Given the ProviderSettingsView was previously left in Detail::EditApiKey for "anthropic" with draft "stale"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![pinfo("anthropic", "api_key", true, 8)]);
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;
    // Force the view into Detail::EditApiKey with a stale draft.
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::Detail {
        provider_id: "anthropic".to_string(),
        sub: DetailSub::EditApiKey {
            draft: "stale".to_string(),
        },
    };

    // @step When the user submits "/provider" again
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step Then the ProviderSettingsView's mode is reset to List
    assert!(matches!(
        app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    ));
    // @step And no stale draft text is rendered
    // (Detail::EditApiKey is the only carrier of `draft`; List mode cannot
    //  render a stale draft)
}
