@done
@RPC-054
@rust
@tui
@app
@dispatch
@provider-settings
Feature: ProviderSettingsView — App dispatch wiring for the /provider slash command
  """
  Wiring from the SlashCommandAction::Provider variant through
  App::dispatch_provider_settings round-trips into the backend, back through Action
  variants, and into the ProviderSettingsView state owned by the
  Navigator. This file covers the dispatch surface; the in-view key
  handling is in rpc054-provider-settings-view.feature.

  The dispatch surface mirrors the RPC-049 / RPC-050 / RPC-053 pattern:
  spawn a tokio task, await the backend round-trip, route the response
  back via Action variants, fold into the view on the App task.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want /provider to open ProviderSettingsView, list credentials,
    save/test/refresh/delete via the backend, and roll responses back
    into the view
    So that the round-trip parity with the TS Ink frontend is preserved
    while the Rust frontend uses its own SessionManagerHandle + tarpc
    transport

  @slash
  @open
  Scenario: /provider slash command opens ProviderSettingsView
    Given a fresh AppTestHarness with focused session s-1
    And the ViewMode is currently Agent
    When the user submits "/provider" via the slash command palette
    Then SlashCommandAction::Provider is dispatched
    And Action::OpenProviderSettingsView is sent on the action bus
    And the Navigator's active_view is ViewMode::ProviderSettings
    And backend.list_provider_credentials is awaited and the response is routed through Action::ProviderCredentialsLoaded
    And the ProviderSettingsView's providers field is populated with the loaded list

  @slash
  @open
  @no-alias
  Scenario: /providers (plural) is NOT a slash command
    Given a fresh AppTestHarness with focused session s-1
    When the user types "/providers" into the input and presses Enter
    Then the SLASH_COMMANDS registry has no entry matching "providers"
    And no SlashCommandAction::Providers variant exists
    And the text "/providers" is sent to the agent as ordinary input (NOT intercepted by the slash dispatcher)
    And the ViewMode stays Agent (no flip to ProviderSettings)

  @slash
  @open
  Scenario: Re-opening /provider resets the view to a clean List mode
    Given the ProviderSettingsView was previously left in Detail::EditApiKey for "anthropic" with draft "stale"
    When the user submits "/provider" again
    Then the ProviderSettingsView's mode is reset to List
    And no stale draft text is rendered

  @esc
  @close
  Scenario: Esc returns from ProviderSettingsView to AgentView
    Given the ProviderSettingsView is open
    When the user presses Esc in List mode
    Then Action::CloseProviderSettingsView is dispatched
    And the Navigator's active_view is ViewMode::Agent
    And the AgentView's prior session, scrollback, and input are intact

  @save
  Scenario: Saving an API key fires backend.set_provider_credentials and refreshes the list
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-test-1"
    When the user presses Enter
    Then Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-test-1" } is dispatched
    And backend.set_provider_credentials("anthropic", ProviderCredentialInput::api_key("sk-test-1")) is awaited
    And on Ok the action Action::ProviderSettingsStatus("✓ anthropic credentials saved") is dispatched
    And a follow-up backend.list_provider_credentials() refresh is dispatched
    And the resulting Action::ProviderCredentialsLoaded folds the new list into the view

  @test-connection
  Scenario: Pressing t inside Detail::Summary runs a connection test
    Given the ProviderSettingsView is in Detail::Summary for "openai"
    When the user presses "t"
    Then Action::TestProviderConnection("openai") is dispatched
    And backend.test_provider_connection("openai") is awaited
    And on Ok the action Action::ProviderTestComplete { provider_id: "openai", result: TestConnectionResult { success: true, latency_ms: 42, .. } } is dispatched
    And the view's last_status updates to TestOk { latency_ms: 42 }

  @test-connection
  Scenario: Backend test_provider_connection error surfaces inline as ✗
    Given the ProviderSettingsView is in Detail::Summary for "openai"
    When the user presses "t"
    And backend.test_provider_connection returns Err("unreachable: dns")
    Then Action::ProviderSettingsStatus("✗ unreachable: dns") is dispatched
    And the view's last_status updates to Error { message: "unreachable: dns" }
    And NO panic occurs
    And NO scrollback notice is emitted to the AgentView

  @refresh-models
  Scenario: Pressing r inside Detail::Summary refreshes the model cache
    Given the ProviderSettingsView is in Detail::Summary for "openai"
    When the user presses "r"
    Then Action::RefreshProviderModels("openai") is dispatched
    And backend.refresh_models_cache("openai") is awaited
    And on Ok the action Action::ProviderModelsRefreshed { provider_id: "openai", model_count: 8 } is dispatched
    And a follow-up backend.list_provider_credentials() refresh is dispatched
    And the openai row's model_count repaints from 4 to 8

  @delete
  @confirm-dialog
  Scenario: d on a configured row opens ConfirmDialog before the backend is called
    Given the ProviderSettingsView is in List mode with "anthropic" focused (configured = true)
    When the user presses "d"
    Then the ConfirmDialog is mounted
    And NO Action::DeleteProviderCredentials nor Action::ConfirmDeleteProviderCredentials is dispatched
    And backend.delete_provider_credentials is NEVER called

  @delete
  @confirm-dialog
  Scenario: Enter on ConfirmDialog Primary fires backend.delete_provider_credentials
    Given the ProviderSettingsView's ConfirmDialog is open for "anthropic" with Primary focused
    When the user presses Enter
    Then Action::ConfirmDeleteProviderCredentials("anthropic") is dispatched
    And backend.delete_provider_credentials("anthropic") is awaited
    And on Ok the action Action::ProviderSettingsStatus("✓ anthropic credentials cleared") is dispatched
    And a follow-up backend.list_provider_credentials() refresh is dispatched
    And the anthropic row repaints with configured = false and model_count = 0

  @delete
  @confirm-dialog
  Scenario: Esc on ConfirmDialog cancels without backend round-trip
    Given the ProviderSettingsView's ConfirmDialog is open for "anthropic"
    When the user presses Esc
    Then the ConfirmDialog is dismissed
    And NO Action::ConfirmDeleteProviderCredentials is dispatched
    And backend.delete_provider_credentials is NEVER called

  @errors
  Scenario: Backend list_provider_credentials error logs via tracing only
    Given the ProviderSettingsView has just been opened
    When backend.list_provider_credentials returns Err("io: disk")
    Then a tracing::warn event is emitted with error = "io: disk"
    And Action::ProviderSettingsStatus("✗ list failed: io: disk") is dispatched
    And NO panic occurs
    And NO scrollback notice is emitted to the AgentView
