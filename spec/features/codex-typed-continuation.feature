@testing
@providers
@persistence
@provider-abstraction
@PROV-149
Feature: Codex typed continuation
  """
  Codex retains typed replay items separately from readable rig history.
  Successful terminal output commits ordered items. Opaque reasoning is preserved
  without becoming display text or diagnostic output. Other providers do not
  acquire Codex state. Backend reload and reset contracts are specified separately
  in codex-session-continuation.feature.
  """

  Scenario: Tool continuation preserves typed output independently of readable thinking
    Given a Codex response with encrypted reasoning phased messages and parallel function calls
    When tool results continue the conversation with readable thinking disabled
    Then the next request preserves ordered typed items and opaque reasoning exactly
    And each tool result matches its original call without duplicate calls

  Scenario: Opaque reasoning round trips without disclosure
    Given arbitrary synthetic opaque reasoning payloads
    When typed checkpoints are serialized and restored
    Then every payload is preserved exactly without appearing in debug output
