@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Recover acknowledged streaming output without repeating side effects
  """
  Version 1 private transcript deltas carry session, request, attempt, message,
  call, sequence and original timestamp identities. Durable admission precedes
  publication of text and thinking and precedes tool dispatch. A failed admission
  stops publication and dispatch and uses the shared bounded recovery owner.
  Deltas never rewrite history. Aborted attempt text is not committed content.
  Promotion references immutable original message IDs; recovery suppresses a
  partial message already present in originals even if acknowledgement failed.
  Requested but undispatched calls are not-started; dispatched calls without an
  authentic captured result are outcome-unknown, never successful or cancelled.
  Recovery is display-only and never schedules tools or changes provider history.
  """

  Scenario: Requested and captured tool boundaries remain truthful
    Given requested, dispatched and result-captured invocations
    When the private transcript store is reopened repeatedly
    Then each invocation retains its authentic not-started, outcome-unknown or completed state

  Scenario: Aborted response attempt text never becomes committed content
    Given a response attempt with visible text that is subsequently aborted
    When another attempt succeeds and the transcript is reopened
    Then only the successful attempt contributes assistant text

  Scenario: Original message promotion suppresses duplicate partial content
    Given a journaled assistant prefix whose immutable original message was committed
    When the process stops before journal promotion acknowledgement
    Then recovery suppresses the partial prefix using the original message identity

  Scenario: Rapid deltas have linear storage amplification
    Given a stream of many fixed-size text deltas
    When every delta is durably acknowledged
    Then stored bytes grow linearly without serializing earlier history
