@done
@BUG-275
@phase-1
@agent-core
@session-management
@persistence
@interrupt
Feature: Make Escape acknowledgement independent of expensive cancellation persistence
  """
  Implement the attached ACDD game plan. Preserve settlement ownership; moving work to spawn_blocking or setting Idle early is not sufficient. Coordinate shared stream-loop changes with BUG-272/276/277.
  Nonterminal acknowledgement bypasses durable settlement. The blocking emitter retains session-owned settlement admission even when its outer future is abandoned; Idle and inference cannot overtake that ownership. Final interruption, completed results, recovery identities, buffer flush, synchronization and atomic promotion remain authoritative.
  Suggested execution order: complete BUG-272 replay preservation, fix BUG-276 lost startup interrupts, implement BUG-277 worker cancellation, then finalize BUG-275 settlement/index optimization and joint profiling. These are related fixes, not artificial blocking dependencies; parallel work requires explicit file ownership.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Escape acknowledgement is not delayed by persistence; safe durable settlement still gates the next turn and successful completion.
  #   2. Cancellation preserves real completed outcomes and records unresolved tool outcomes truthfully exactly once, including crash/reopen and persistence failures.
  #   3. Index publication uses bounded buffered writes while retaining compatible layout, synchronization, atomic promotion and integrity checks.
  #
  # EXAMPLES:
  #   1. Hold cancellation persistence at a barrier, press Escape, and observe acknowledgement before release while a new inference turn remains inadmissible.
  #   2. Cancel an executing tool with a large synthetic index; settle and cold reopen to find one original assistant call and one truthful outcome without per-entry unbuffered writes.
  #
  # ========================================
  Background: User Story
    As a session owner
    I want to see interruption acknowledgement immediately while durable settlement remains safe
    So that I can stop work without losing truthful outcomes or admitting overlapping turns

  Scenario: Acknowledgement precedes durable cancellation settlement
    Given a running tool whose cancellation persistence is paused
    When the user interrupts the turn
    Then visible acknowledgement arrives before persistence is released
    And unrelated async work remains responsive
    And neither idle successful completion nor another inference turn overtakes settlement

  Scenario: Settlement retains truthful tool outcomes exactly once
    Given one completed tool and one unresolved running tool
    When the turn is interrupted and safely settled
    Then the completed outcome including genuine errors is preserved
    And the unresolved tool has one truthful cancelled or unknown outcome
    And cold reopen preserves original identities and ordering without duplicates

  Scenario: Failed settlement remains recoverable
    Given a cancellation whose required persistence fails
    When the session is reopened and persistence is retried
    Then original input calls and outcomes remain recoverable exactly once
    And no false success or new inference bypasses the unsettled records

  Scenario: Index publication uses bounded buffered writes
    Given a large synthetic message index
    When a cancellation outcome publishes that index
    Then filesystem writes are buffered in bounded batches rather than per entry
    And compatible index bytes are flushed before synchronization and atomic promotion

  Scenario: Publication failures retain crash-safe history
    Given an index publication with injected write flush sync rename or directory failure
    When publication fails and storage is cold reopened
    Then committed originals remain readable with intact identities and hashes
    And incomplete publication never masquerades as durable successful settlement
