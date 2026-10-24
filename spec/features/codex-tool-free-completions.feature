@testing
@providers
@integration
@provider-abstraction
@PROV-147
Feature: Codex tool-free completions
  """
  Background extraction must use the same Codex boundary as interactive agents,
  without registering tools. Subscription direct completions are buffered from
  the streaming-only endpoint while API-key JSON completions remain unchanged.
  """

  Scenario: Tool-free Codex callers use the compatibility boundary
    Given a Codex provider used for background learnings extraction
    When a tool-free completion model is requested
    Then it uses the same Codex-owned boundary as the production agent
    And its requests disable storage and include encrypted reasoning

  Scenario: Subscription direct completions buffer streaming responses
    Given a Codex subscription endpoint that only accepts streaming requests
    When a direct completion is requested through the Codex boundary
    Then the request uses streaming with storage disabled
    And the completed response retains its text and token usage
