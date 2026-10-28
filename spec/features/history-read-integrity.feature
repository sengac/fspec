@done
@BUG-277
@phase-1
@agent-core
@session-management
@persistence
@interrupt
Feature: History read integrity during cancellation
  """
  Add invocation-scoped cooperative cancellation and a drop guard to the spawn_blocking path; bound inner read batches and lease waits, not only outer session iteration. Retain BUG-273 offloading and generation/integrity validation. Follow the attached RED-first plan.
  Actual Rig ToolServer executes tools in a service task after a request caller drops. Extend the existing tool trait with an opt-in cancel_on_abandon policy (default false), enabled only for read-only SessionSearch. Select cancellation of the invocation reply channel before polling an opted-in queued call and during its execution, so the search drop guard reaches the blocking worker. Keep default mutating-tool execution and genuine errors intact; cover direct/server abandonment, a queued abandoned search, and a non-opted-in counter/latch control. No global interrupt flag or universal tool abort.
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
  #   3. After I cancel an initial history read, a later history listing still includes every healthy session without a restart, and healthy files are never reported as persistently unreadable.
  #   4. A history read encountering a transient operating-system interruption still returns the complete result, but an actual user cancellation stops it instead of being retried forever.
  #
  # ========================================
  Background: User Story
    As a session owner
    I want to cancel or abandon history searches without leaving workers behind
    So that unrelated history access and settlement can progress safely

  Scenario: Cancelled cold discovery never hides healthy sessions
    Given healthy session manifests awaiting their first history discovery
    When cancellation interrupts the last manifest read
    Then a subsequent listing without resetting storage retains every original session
    And healthy files remain unchanged without persistent unreadable diagnostics

  Scenario: Transient operating system interruptions preserve successful reads
    Given readers that receive a transient operating system interruption
    When complete and delimiter-bounded history reads resume
    Then both return their complete original bytes
    And actual invocation cancellation still stops retries at the next checkpoint
