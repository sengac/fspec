@done
@BUG-277
@phase-1
@agent-core
@session-management
@persistence
@interrupt
Feature: Stop cancelled SessionSearch workers and bound shared-storage lock occupancy
  """
  Add invocation-scoped cooperative cancellation and a drop guard to the spawn_blocking path; bound inner read batches and lease waits, not only outer session iteration. Retain BUG-273 offloading and generation/integrity validation. Follow the attached RED-first plan.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Cancelling or abandoning an active SessionSearch invocation stops its actual worker at a bounded checkpoint and releases storage ownership without publishing a stale success.
  #   2. Search uses bounded storage-lock occupancy so cancellation and unrelated session reads/writes can progress without waiting for a complete large-history scan.
  #   3. Uncancelled search semantics, limits, trimming, originals integrity, hook denial and single-worker async responsiveness remain unchanged.
  #
  # EXAMPLES:
  #   1. The user cancels a search through a large history; background scanning stops rather than continuing after the session becomes idle, and another session can save its result.
  #   2. A tool invocation is abandoned while its search worker is running; the worker releases storage and never publishes an old result into the next turn.
  #
  # ========================================
  Background: User Story
    As a session owner
    I want to cancel or abandon history searches without leaving workers behind
    So that unrelated history access and settlement can progress safely

  Scenario: Abandoned search releases a contended storage lease
    Given an active history search waiting for storage ownership
    When its tool future is dropped while storage remains unavailable
    Then the actual worker stops without waiting for storage release
    And it publishes no successful result into another turn

  Scenario: Cancellation stops reads within a large session
    Given a history search paused between bounded message batches
    When its invocation is cancelled
    Then no additional history batch is read
    And an unrelated session can persist while the search is paused

  Scenario: Cancellation covers startup and session boundaries
    Given history searches queued for execution or moving between sessions
    When their invocation is cancelled at either boundary
    Then expensive history work is not started or resumed
    And cancellation is explicit rather than an empty successful search

  Scenario: Cancellation interrupts large original and blob reads
    Given a search resolving a large stored original or blob
    When its invocation is cancelled between bounded byte reads
    Then further reads stop and all storage ownership is released
    And no partial original or blob is published as successful evidence

  Scenario: Uncancelled history access preserves its contracts
    Given real history with blobs and multiple sessions
    When uncancelled search show recent and originals actions run
    Then filtering trimming limits routing and integrity checks remain intact
    And hook denial prevents dispatch while unrelated async work remains responsive
