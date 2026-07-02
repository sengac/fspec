@done
@agent-view
@ts-parity
@provider-settings
@rust
@tui
@RPC-163
Feature: Provider settings: Delete key (in addition to Backspace) in API-key edit form
  """
  TS reference: src/tui/components/ProviderSettingsPanel.tsx uses Ink's useInput, where key.backspace and key.delete are sibling boolean flags both wired to draft.slice(0, -1). Rust impl currently only binds KeyCode::Backspace at codelet/fspec-tui/src/views/provider_settings/detail.rs:139-146.
  Summary/OAuthNotice sub-modes already route to handle_summary_key / handle_oauth_notice_key — Delete falls into the catch-all `_` arm there and preserves state (no change required, but explicit guard scenario required).
  Implementation:
  - merge KeyCode::Delete into the existing Backspace arm via `KeyCode::Backspace | KeyCode::Delete => { draft.pop(); ... }` — single shared body guarantees no behavioural divergence (Rule 3).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing KeyCode::Delete in EditApiKey sub-mode pops the last character of the draft buffer, identical to KeyCode::Backspace
  #   2. Pressing KeyCode::Delete on an empty draft is a silent no-op (draft remains empty, no validation error raised)
  #   3. KeyCode::Delete and KeyCode::Backspace share the same match arm — neither key path may diverge in behaviour
  #   4. KeyCode::Delete in Summary or OAuthNotice sub-modes is treated as an unrelated key (no state mutation beyond preserving current sub-mode)
  #   5. Pressing KeyCode::Delete must NOT clear the 'API key cannot be empty' validation status — only an accepted printable char clears it
  #
  # EXAMPLES:
  #   1. Draft is "abc123"; user presses Delete; draft becomes "abc12" and view stays in EditApiKey
  #   2. Draft is empty; user presses Delete; draft remains empty and no validation error appears
  #   3. Draft is "x"; user presses Delete; draft becomes empty string and view stays in EditApiKey
  #   4. Draft is "hello"; user alternates Backspace, Delete, Backspace, Delete, Backspace; draft becomes empty (both keys produce identical pop)
  #   5. Draft is "abc", status is "API key cannot be empty" (e.g. from a prior empty-Enter); user presses Delete; draft becomes "ab" AND status text remains "API key cannot be empty"
  #   6. View is in Summary sub-mode for provider 'anthropic'; user presses Delete; view remains in Summary with last_status preserved (no draft buffer affected)
  #   7. View is in OAuthNotice sub-mode; user presses Delete; view remains in OAuthNotice (only Esc exits)
  #
  # ========================================
  Background: User Story
    As a developer testing the Rust ProviderSettings TUI against TS parity
    I want to press Delete (in addition to Backspace) while editing an API key
    So that I can delete the last character of my draft using either key, matching the TS frontend's useInput dual-binding

  Scenario: Pressing Delete on a multi-character draft pops the last character
    Given I am in the EditApiKey form with the draft "abc123"
    When I press the Delete key
    Then the draft becomes "abc12"
    And the view remains in Detail::EditApiKey for the same provider
    And the keystroke is reported as ProviderSettingsEvent::Consumed
    And no Action is dispatched

  Scenario: Pressing Delete on an empty draft is a silent no-op
    Given I am in the EditApiKey form with an empty draft
    And the inline validation status is empty
    When I press the Delete key
    Then the draft remains empty
    And the inline validation status remains empty
    And the view remains in Detail::EditApiKey for the same provider
    And the keystroke is reported as ProviderSettingsEvent::Consumed

  Scenario: Pressing Delete on a single-character draft empties it
    Given I am in the EditApiKey form with the draft "x"
    When I press the Delete key
    Then the draft becomes ""
    And the view remains in Detail::EditApiKey for the same provider

  Scenario: Backspace and Delete produce identical pops when alternated
    Given I am in the EditApiKey form with the draft "hello"
    When I press the following keys in order: Backspace, Delete, Backspace, Delete, Backspace
    Then the draft is empty after each step matches the same sequence "hell", "hel", "he", "h", ""
    And the final draft is ""
    And the view remains in Detail::EditApiKey for the same provider throughout
    And every keystroke is reported as ProviderSettingsEvent::Consumed

  Scenario: Pressing Delete must not clear the "API key cannot be empty" status
    Given I am in the EditApiKey form with the draft "abc"
    And the inline validation status is "API key cannot be empty"
    When I press the Delete key
    Then the draft becomes "ab"
    And the inline validation status remains "API key cannot be empty"
    And the view remains in Detail::EditApiKey for the same provider

  Scenario: Pressing Delete in Summary sub-mode is treated as unrelated
    Given I am in Detail::Summary for the provider "anthropic" with last_status Some(Testing)
    When I press the Delete key
    Then the view remains in Detail::Summary for "anthropic"
    And the last_status remains Some(Testing)
    And the keystroke is reported as ProviderSettingsEvent::Consumed
    And no Action is dispatched

  Scenario: Pressing Delete in OAuthNotice sub-mode does not exit the notice
    Given I am in Detail::OAuthNotice for an OAuth-only provider
    When I press the Delete key
    Then the view remains in Detail::OAuthNotice
    And the keystroke is reported as ProviderSettingsEvent::Consumed
