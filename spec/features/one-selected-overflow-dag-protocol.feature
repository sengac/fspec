@cli
@context-management
@compaction
@CMPCT-058
Feature: One selected overflow DAG protocol
  """
  Typed provider overflow admission is independent of error wording. All admitted
  routes execute the same in-view agent-owned DAG protocol and validator contract.
  Macro evaluation order and retry count cannot select a different builder.
  """

  Scenario: Equivalent typed overflow routes select the same protocol
    Given selected OpenAI and Anthropic adapters receiving structured overflow rejections
    When each adapter executes standalone streaming overflow recovery
    Then every retry sends the same in-view DAG construction instruction
    And no substitute completion occurs and ordinary inference stays blocked
    And committed conversation and durable original envelopes remain unchanged

  Scenario: Unknown token-like errors do not select compaction
    Given an unrecognized error containing token-like wording
    When overflow recovery routes the error
    Then no DAG construction protocol is selected
