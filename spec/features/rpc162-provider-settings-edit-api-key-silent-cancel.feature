@done
@RPC-162
@agent-view
@ts-parity
@provider-settings
@tui
Feature: Provider settings: empty-Enter in API-key edit cancels silently (drop inline 'cannot be empty' validation)
  """
  TS reference: src/tui/components/ProviderSettingsPanel.tsx — the editApiKey form's Enter handler returns silently when draft is empty (no validation chip rendered). On successful save, the form closes and the panel returns to the provider list (Ink unmounts the form node and re-renders the list). No "summary" intermediate view exists in TS.
  Test impact: rust/fspec-tui/tests/provider_settings_view_rpc054.rs contains pre-existing tests that pin the OLD Esc→Summary and Enter→Summary { SavingCredentials } behavior. Those scenarios must be deleted from spec/features/rpc054-provider-settings-view.feature and their test bodies removed from provider_settings_view_rpc054.rs as part of this card — they are superseded by the new RPC-162 scenarios.
  Implementation:
  - rust/fspec-tui/src/views/provider_settings/detail.rs handle_edit_key — Enter arm with empty draft now sets view.mode = ProviderSettingsMode::List, clears view.status, and returns ProviderSettingsEvent::Consumed (no Action). Esc arm now sets view.mode = ProviderSettingsMode::List (was Detail::Summary { last_status: None }). Enter arm with non-empty draft now sets view.mode = ProviderSettingsMode::List before emitting Action::SaveProviderCredentials (was Detail::Summary { SavingCredentials }). The DetailSub::Summary variant and DetailStatus enum remain untouched for legacy callers and existing tests.
  """

  Background: User Story
    As a fspec TUI user editing an API key
    I want to press Enter on an empty draft to abandon the form
    So that I can back out of accidental edits without seeing inline validation chrome, matching the TS frontend

  @detail
  @edit
  @silent-cancel
  Scenario: Pressing Enter on an empty EditApiKey draft transitions to List mode and emits no Action
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    When the user presses Enter
    Then the view's mode is ProviderSettingsMode::List
    And view.status is the empty string
    And no ProviderSettingsEvent::Emit is dispatched
    And handle_key returns ProviderSettingsEvent::Consumed

  @detail
  @edit
  @silent-cancel
  Scenario: Pressing Enter on an empty EditApiKey draft never writes the legacy "API key cannot be empty" status
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    And view.status is the empty string
    When the user presses Enter
    Then view.status remains the empty string
    And view.status is never equal to "API key cannot be empty"

  @detail
  @edit
  @save
  Scenario: Pressing Enter on a non-empty EditApiKey draft emits SaveProviderCredentials and returns to List mode
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-abc"
    When the user presses Enter
    Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-abc" })
    And the view's mode is ProviderSettingsMode::List
    And view.status is the empty string

  @detail
  @edit
  @esc-hierarchy
  Scenario: Pressing Esc in EditApiKey transitions directly to List mode
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-cancel"
    When the user presses Esc
    Then the view's mode is ProviderSettingsMode::List
    And view.status is the empty string
    And no ProviderSettingsEvent::Emit is dispatched
    And handle_key returns ProviderSettingsEvent::Consumed

  @detail
  @edit
  @silent-cancel
  Scenario: Pressing Esc in EditApiKey with an empty draft also returns directly to List mode
    Given the ProviderSettingsView is in Detail::EditApiKey for "openai" with empty draft
    When the user presses Esc
    Then the view's mode is ProviderSettingsMode::List
    And view.status is the empty string

  @detail
  @edit
  @silent-cancel
  Scenario: Empty-Enter cancel after typing then deleting all characters still produces no validation chrome
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    When the user types "sk-"
    And the user presses Backspace 3 times so the draft is empty again
    And the user presses Enter
    Then the view's mode is ProviderSettingsMode::List
    And view.status is the empty string

  @detail
  @edit
  @silent-cancel
  @regression
  Scenario: Empty-Enter clears any pre-existing legacy "API key cannot be empty" status
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    And view.status has been manually set to "API key cannot be empty" (legacy state)
    When the user presses Enter
    Then view.status is the empty string
    And the view's mode is ProviderSettingsMode::List

  @detail
  @edit
  @save
  Scenario: Non-empty Enter still consumes the draft into the SaveProviderCredentials payload verbatim
    Given the ProviderSettingsView is in Detail::EditApiKey for "openai" with draft "sk-test-1"
    When the user presses Enter
    Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "openai", api_key: "sk-test-1" })
    And the view's mode is ProviderSettingsMode::List
