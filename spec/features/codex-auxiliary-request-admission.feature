@testing
@agent-core
@providers
@provider-abstraction
Feature: Codex auxiliary request admission
  """
  DeepSearch and compaction use ephemeral agents with restricted tool catalogs.
  Their Codex requests must cross the same provider-owned compatibility and
  physical admission boundary as ordinary Codex requests, without acquiring
  the main agent's broader tool catalog.
  """

  Scenario: DeepSearch rejects missing Codex limits before transport
    Given a DeepSearch task using a Codex model without complete admission limits
    When its ephemeral agent requests a streamed completion
    Then an explicit admission failure is returned
    And no request reaches the transport

  Scenario: Compaction rejects missing Codex limits before transport
    Given a compaction task using a Codex model without complete admission limits
    When its ephemeral agent requests a streamed completion
    Then an explicit admission failure is returned
    And no request reaches the transport

  Scenario: DeepSearch rejects an oversized Codex request before transport
    Given a DeepSearch task whose full request exceeds the inherited Codex capacity
    When its ephemeral agent requests a streamed completion
    Then an explicit capacity failure is returned
    And no request reaches the transport

  Scenario: Compaction rejects an oversized Codex request before transport
    Given a compaction task whose full request exceeds the inherited Codex capacity
    When its ephemeral agent requests a streamed completion
    Then an explicit capacity failure is returned
    And no request reaches the transport
