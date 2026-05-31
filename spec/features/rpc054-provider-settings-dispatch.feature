@done
@session-management
@provider-settings
@agent-view
@rpc
@tui
@rust
@RPC-054
Feature: /provider ProviderSettingsView App-dispatch routing (Action flow)

  """
  App-level dispatch behaviour for the ProviderSettingsView surface — Action::OpenProviderSettingsView / CloseProviderSettingsView / SaveProviderCredentials / TestProviderConnection / RefreshProviderModels / DeleteProviderCredentials flow through the App's tokio task chain against a scripted MockBackend. The synchronous keyboard surface lives in rpc054-provider-settings-view.feature; transport parity lives in rpc054-provider-settings-cross-transport-parity.feature.
  """

  Background: User Story
    As a user of the Rust ratatui AgentView
    I want /provider to open a settings view that lists configured providers and lets me edit credentials, test connections, refresh models, and clear credentials
    So that I can configure and verify LLM provider credentials end-to-end through the App's action bus without leaving the TUI

  # ─────────────────────────────────────────────────────────────────────
  # /provider slash command opens the view
  # ─────────────────────────────────────────────────────────────────────
  Scenario: /provider slash command opens ProviderSettingsView
    Given an App with a MockBackend
    And the MockBackend's list_provider_credentials is scripted to return [anthropic api_key configured 8 models, openai api_key not_configured 0 models]
    When the user submits "/provider" via the slash command palette
    And all pending tasks have drained
    Then the Navigator's active view is ProviderSettings
    And the ProviderSettingsView's provider list contains 2 rows
    And the focused row is "anthropic" with configured indicator "✓" and model count 8

  Scenario: Esc returns from ProviderSettingsView to AgentView
    Given the ProviderSettingsView is open in list mode
    And the previously focused session is s-1
    When the user presses Esc
    Then the Navigator's active view is Agent
    And the current session is s-1

  # ─────────────────────────────────────────────────────────────────────
  # API key editing flow
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Enter on the API key edit form saves and refreshes the list
    Given the ProviderSettingsView is in edit-api-key mode for "anthropic" with draft "sk-test"
    When the user presses Enter
    And all pending tasks have drained
    Then backend.set_provider_credentials is called exactly once with provider_id "anthropic" and an ApiKey input with key "sk-test"
    And backend.list_provider_credentials is called at least once after the save
    And the ProviderSettingsView is back in list mode
    And the anthropic row shows configured indicator "✓"

  # ─────────────────────────────────────────────────────────────────────
  # Test connection
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Pressing 't' on a row runs a connection test
    Given the ProviderSettingsView is open with the openai row focused
    And the MockBackend's test_provider_connection is scripted to return TestConnectionResult{ success: true, error: None, latency_ms: 42 } for "openai"
    When the user presses "t"
    And all pending tasks have drained
    Then backend.test_provider_connection is called exactly once with "openai"
    And the right-pane status area shows "✓ ok (42ms)"

  Scenario: Pressing 't' surfaces backend errors inline
    Given the ProviderSettingsView is open with the openai row focused
    And the MockBackend's test_provider_connection is scripted to return TestConnectionResult{ success: false, error: Some("unreachable: dns resolution failed"), latency_ms: 0 } for "openai"
    When the user presses "t"
    And all pending tasks have drained
    Then the right-pane status area contains "✗ unreachable: dns resolution failed"
    And the openai row's configured indicator is unchanged

  # ─────────────────────────────────────────────────────────────────────
  # Refresh models
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Pressing 'r' refreshes the model list and updates the row count
    Given the ProviderSettingsView is open with the openai row focused
    And the openai row's model count is 4
    And the MockBackend's refresh_models_cache is scripted to return a 8-entry model list for "openai"
    And the MockBackend's list_provider_credentials is scripted to return [openai api_key configured 8 models] after the refresh
    When the user presses "r"
    And all pending tasks have drained
    Then backend.refresh_models_cache is called exactly once with "openai"
    And backend.list_provider_credentials is called at least once after the refresh
    And the openai row's model count is 8
    And the right-pane status area contains "models refreshed"

  # ─────────────────────────────────────────────────────────────────────
  # Delete provider credentials
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Pressing 'd' on a configured row clears the credentials
    Given the ProviderSettingsView is open with the anthropic row focused
    And the anthropic row is configured
    And the MockBackend's list_provider_credentials is scripted to return [anthropic api_key not_configured 0 models] after the delete
    When the user presses "d"
    And all pending tasks have drained
    Then backend.delete_provider_credentials is called exactly once with "anthropic"
    And backend.list_provider_credentials is called at least once after the delete
    And the anthropic row shows configured indicator "(not configured)"
    And the anthropic row's model count is 0

  # ─────────────────────────────────────────────────────────────────────
  # Error tolerance
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Backend errors are silently logged without panicking
    Given the ProviderSettingsView is open with the anthropic row focused
    And the MockBackend's set_provider_credentials is scripted to return Err("write failed")
    When the user opens the API-key edit form for anthropic, types "sk-test", and presses Enter
    And all pending tasks have drained
    Then the App must not panic
    And no scrollback chunks contain the text "set_provider_credentials"
    And the right-pane status area contains "✗ write failed"
