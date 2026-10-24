@agent-core
@phase-1
@context-management
@compaction
@CMPCT-056
Feature: Deferred cross-session application retains captured candidate identity
  """
  The cross-session-compaction-ownership contract applies. Deferred application
  cannot redirect a submitted candidate or promote after cancellation. Retain
  durable originals, block ordinary inference and never fabricate completion.
  """

  Scenario: Deferred application cannot redirect a captured candidate
    Given a candidate submitted for a captured live target
    When deferred application names a different live target
    Then application rejects without promoting either durable session
    And the candidate remains available only to its captured target

  Scenario: Cancellation after submission cannot promote the pending candidate
    Given a candidate submitted for a captured live target
    When the target is interrupted before deferred application
    Then application returns cancellation without promoting durable context
    And inference remains blocked without completion or changed counters
