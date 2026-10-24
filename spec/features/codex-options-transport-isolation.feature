@PROV-152
@providers
@provider-abstraction
@integration
Feature: Codex options transport isolation
  """
  Only Codex resolves capability-aware options; identical model names do not
  change ordinary OpenAI serialization or required Codex request invariants.
  """

  Scenario: Non-Codex requests retain their existing configuration
    Given Codex and ordinary OpenAI models have the same model name
    When each provider sends its configured request
    Then only Codex applies capability resolution
    And ordinary OpenAI request serialization remains unchanged
