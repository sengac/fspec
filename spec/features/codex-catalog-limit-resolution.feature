@testing
@session
@model-selection
@compaction
Feature: Codex selections retain catalog model limits
  """
  The model selector synthesizes Codex models from the OpenAI models.dev
  catalog. Shared selection resolution must use that same catalog source for
  context and output limits while retaining Codex as the transport identity.
  Missing metadata must stay missing rather than introducing guessed limits.
  """

  Background: User story
    As a Codex session owner
    I want model selection to retain the catalog capacity
    So that ordinary prompts do not trigger zero-threshold compaction

  Scenario: Creating a Codex session resolves limits from its source catalog
    Given the OpenAI catalog contains a model offered through Codex
    When a session is created with that Codex model
    Then active and cached limits match the model catalog
    And its compaction threshold is positive
    And its selected transport remains Codex

  Scenario: Switching Codex models refreshes limits from the source catalog
    Given a Codex session with resolved model limits
    When the session switches to another catalog-backed Codex model
    Then active and cached limits match the new model catalog
    And its compaction threshold follows the new capacity
    And its selected transport remains Codex

  Scenario: An uncatalogued Codex selection does not inherit guessed limits
    Given a Codex session with resolved model limits
    When the session switches to a model absent from the catalog
    Then active and cached limits are unavailable
    And no limits from the previous model are retained

  Scenario: A partial Codex model name does not authorize catalog capacity
    Given a Codex session with resolved model limits
    When the session switches to a name that only prefixes a catalog model
    Then active and cached limits are unavailable
    And no limits from a different model are used
