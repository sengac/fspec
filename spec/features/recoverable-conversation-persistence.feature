@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Recover conversation persistence without losing authentic outcomes
  """
  One private, versioned, ordered recovery spool owns stable message envelopes
  until both the message and its manifest reference are acknowledged. Retries
  reuse physical identities, reconcile ambiguous promotion, and never dispatch
  tools or lifecycle hooks. Three immediate attempts precede visible recovery
  required state. Explicit retry or the next admitted input drains pending work.
  Admission is bounded to 128 envelopes and 16 MiB per session. Exhaustion
  refuses further inference and side effects; accepted content is not evicted.
  If even recovery storage is unavailable the original in-memory owner retains
  content and a visible error warns that shutdown is unsafe until recovery.
  Streaming-prefix crash recovery and pre-dispatch crash guarantees are BUG-249.
  """

  Background: User Story
    """
    As a user whose session storage becomes unavailable
    I want authentic conversation records retained and recoverable
    So that retry restores my transcript without repeating external actions
    """

  Scenario: Assistant content survives a temporary archive failure
    Given a real session with an unavailable message archive
    When assistant text settles while the archive is unavailable
    And the archive is repaired and persistence is retried
    Then cold reopen contains the original assistant text exactly once

  Scenario: An invocation failure cannot orphan its authentic result
    Given a real session with an unavailable message archive
    When a tool invocation and its authentic result settle
    And the archive is repaired and persistence is retried
    Then cold reopen contains the invocation before its matching result exactly once

  Scenario: Retrying a captured result never repeats its side effect
    Given an archived invocation for a sentinel writing tool
    When its captured result cannot be archived
    And the archive is repaired and persistence is retried
    Then the authentic result is archived once and the sentinel action ran once
