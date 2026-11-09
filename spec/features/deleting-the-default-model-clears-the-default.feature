@BUG-582
@done
@session
@model-selection
@bug-fix
@rust
@BUG-613
Feature: Deleting the default model clears the default
  """
  Product owner, 2026-11-07 (option B): deleting the custom model that is fspec's default model deletes it and clears the default in the same operation, and reports it; deleting a whole profile that holds the default does the same. Profile model strings are provider:profile/model-id. The default lives in SessionManager.default_model, <data dir>/default-model.json (PROV-119) and tui.lastUsedModel (PROV-122); all three are cleared. The RPC replies are unchanged (no wire change): the daemon logs the clear, RPC clients see get_default_model return no model, and the TUI shows a session notice. Creating a session without a default fails with an error naming /model and set_default_model.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Deleting a custom model that is not the default is unchanged
  #   2. Deleting the default custom model, or the profile that holds it, deletes it and clears the default (memory, default-model.json, tui.lastUsedModel)
  #   3. The default is x:p/m for custom model m of profile p of provider x; a profile delete matches defaults starting with x:p/
  #   4. The daemon logs the clear; RPC replies are unchanged; the TUI shows a session notice naming the deleted default and /model
  #   5. Session creation without a default model names how to choose one
  #
  # EXAMPLES:
  #   1. Default openai:local/m1; delete_custom_model(openai, local, m1); get_default_model returns none
  #   2. Default anthropic/claude-opus-4-5; deleting m1 leaves it
  #   3. Default openai:local/m1; deleting profile local clears it; a restarted daemon has no default
  #   4. Afterwards create_session fails naming /model and set_default_model
  #
  # ========================================
  Background: User Story
    As a user managing custom models through the TUI or an RPC client
    I want deleting the model fspec uses as its default to leave no default pointing at it
    So that new sessions are never created on a model that no longer exists

  Scenario: Deleting the default custom model clears the default
    Given the default model is "openai:local/m1", a custom model of profile "local"
    When the custom model m1 of openai profile "local" is deleted
    Then the model is gone from the profile
    And get_default_model returns no model
    And default-model.json and tui.lastUsedModel no longer name "openai:local/m1"

  Scenario: Deleting another custom model leaves the default
    Given the default model is "anthropic/claude-opus-4-5"
    When the custom model m1 of openai profile "local" is deleted
    Then get_default_model still returns "anthropic/claude-opus-4-5"

  Scenario: Deleting the profile that holds the default clears the default
    Given the default model is "openai:local/m1"
    When the openai profile "local" is deleted
    Then get_default_model returns no model
    And a session manager started afterwards has no default model

  Scenario: Creating a session without a default model names how to choose one
    Given the default model was cleared by deleting it
    When a session is created
    Then it fails with an error naming /model and set_default_model

  Scenario: The TUI reports that the deleted model was the default
    Given the TUI's default model is "openai:local/m1"
    When the TUI deletes the custom model m1 of openai profile "local"
    Then the session shows a notice that "openai:local/m1" was the default model and that /model chooses a new one
