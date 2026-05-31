@done
@session-management
@provider-settings
@agent-view
@rpc
@tui
@rust
@RPC-054
Feature: /provider ProviderSettingsView keyboard handling (view-isolation)

  """
  Isolated view-layer behaviour of ProviderSettingsView (codelet/fspec-tui/src/views/provider_settings/mod.rs) — keyboard input → emitted Action / mode transition. No App, no backend; drives the synchronous handle_key surface only. The wider RPC + dispatch + transport plumbing is covered in sibling feature files (rpc054-provider-settings-dispatch.feature, rpc054-provider-settings-cross-transport-parity.feature, rpc054-provider-settings-source-shape.feature).
  """

  Background: User Story
    As a user of the Rust ratatui AgentView
    I want the ProviderSettingsView to respond to keyboard input correctly in isolation
    So that Enter / Esc / t / r / d / typing produce the right mode transitions and emitted Actions before the dispatcher even sees them

  Scenario: Enter on an api_key row opens an inline edit form
    Given the ProviderSettingsView is open with the anthropic row focused
    And the anthropic row's credential_type is "api_key"
    When the user presses Enter
    Then the ProviderSettingsView is in edit-api-key mode for "anthropic"
    And the edit form's draft value is empty

  Scenario: Typing into the API key edit form updates the draft value
    Given the ProviderSettingsView is in edit-api-key mode for "anthropic"
    When the user types "sk-1234abcd" into the edit form
    Then the edit form's draft value is "sk-1234abcd"

  Scenario: Esc on the API key edit form cancels without saving
    Given the ProviderSettingsView is in edit-api-key mode for "anthropic" with draft "sk-cancel"
    When the user presses Esc
    Then backend.set_provider_credentials is NEVER called
    And the ProviderSettingsView is back in list mode

  Scenario: Enter on an OAuth-type row shows the read-only notice
    Given the ProviderSettingsView is open with the codex row focused
    And the codex row's credential_type is "oauth"
    When the user presses Enter
    Then the ProviderSettingsView is still in list mode
    And the right-pane status area contains "OAuth flow not yet supported in Rust frontend"
    And backend.set_provider_credentials is NEVER called
