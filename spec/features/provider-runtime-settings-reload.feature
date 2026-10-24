@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Reload saved provider settings at turn boundaries
  """
  Reload runtime options on the existing provider manager while holding the inner-session lock before dispatch. Do not reselect the model or recreate provider session state. Publish refreshed token limits and tool image budgets together with inherited runtime defaults. Invalid settings fail the dispatch without partially applying options.
  """

  Background: User Story
    As a user editing provider settings during a conversation
    I want my next turn to use the saved settings
    So that existing sessions do not retain stale context limits

  Scenario: The next turn reloads saved settings without replacing the conversation
    Given a selected model with conversation history and inherited provider settings
    When saved provider settings change before the next turn
    Then the next turn uses updated settings and cached model limits without changing history or provider identity

  Scenario: Reloading settings preserves explicit session overrides
    Given a selected model with explicit token limits compaction threshold and continuation choices
    When saved provider settings change before the next turn
    Then explicit session choices remain authoritative while inherited settings refresh

  Scenario: Invalid saved settings leave the current session unchanged
    Given a selected model with valid runtime settings
    When invalid provider settings are loaded at the next turn boundary
    Then dispatch reports the settings error without changing the current settings or cached limits

  Scenario: An existing Codex session reloads its saved context window
    Given a Codex session using gpt-6.1-sol with catalog context limits
    When the Codex context window is saved as 800000 tokens
    Then the next turn publishes the saved context window instead of stale catalog limits
