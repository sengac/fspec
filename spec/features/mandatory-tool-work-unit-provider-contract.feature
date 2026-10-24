@TOOL-025
@providers
@tools
@tool-execution
@schema-design
@rust
Feature: Final provider transports preserve mandatory tool attribution
  """
  Required top-level work_unit_id attribution must survive provider serialization and custom Rhai
  request construction. Strict Codex agents keep patch execution in JSON and
  cannot expose raw or hosted execution without model-supplied attribution.
  """

  Background: User Story
    As an AI agent
    I want every provider to preserve mandatory tool attribution
    So that provider-specific transformations cannot bypass the shared contract

  Scenario: Final provider requests retain the attribution contract
    Given representative OpenAI Anthropic Gemini Codex and custom provider agents
    When the final tool-bearing provider requests are serialized
    Then required harness attribution survives each provider's schema conversion
    And representative strict business argument schemas remain valid

  Scenario: Raw and hosted native tools cannot bypass strict attribution
    Given a Codex agent configured with raw patch and hosted search capabilities
    When strict attributed tools are prepared for the model
    Then patch execution is exposed through a JSON function with mandatory attribution
    And hosted execution lacking an attribution channel is not exposed
    And the harness does not silently synthesize "NONE" for model calls
