@done
@testing
@session
@model-selection
@compaction
@BUG-335
@COV-062
Feature: Codex selections retain catalog model limits
  """
  The model selector synthesizes Codex models from the OpenAI models.dev
  catalog. Shared selection resolution must use that same catalog source for
  context and output limits while retaining Codex as the transport identity.
  Missing metadata must stay missing rather than introducing guessed limits.

  BUG-335: catalog lookup matches a selection only by catalog key or model id,
  exactly. A name that is merely a prefix of one catalog model (gpt-4 against
  gpt-4o) resolves no entry, so it never borrows that model's capacity.

  COV-062: every scenario is linked to its test and implementation, and every
  step is a byte-identical @step comment in
  rust/sessions/tests/codex_catalog_limit_resolution.rs.
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
    Then the switch is refused naming the missing context window and output limit
    And the session keeps its catalogued model and limits

  Scenario: A partial Codex model name does not authorize catalog capacity
    Given a Codex session with resolved model limits
    When the session switches to a name that only prefixes a catalog model
    Then the switch is refused naming the missing context window and output limit
    And the session keeps its catalogued model and limits
