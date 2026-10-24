@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Refresh context state and honest coverage with committed compaction events
  """
  Implementation:
  - P2; CB-17. Read attached cmpct-068-commit-ui-refresh.md and 00-shared-contract.md. Evidence classification and exact source anchors are in the specific brief. No runtime reproduction claimed. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Publish committed generation, context/token/fill snapshot and compaction lifecycle coherently; update background caches and TUI without awaiting another provider call.
  #   3. Report durable coverage accurately or explicitly unknown, never hard-coded measured zero. Distinguish estimated counts and trigger fill from absolute model capacity.
  #   4. Rejected/uncommitted operations produce no success badge/completion; stale events cannot overwrite current counters. Post-commit notification failure reports the new commit identity for reconciliation, never rolls back or fabricates delivery.
  #
  # EXAMPLES:
  #   1. Given a full-looking header, when a valid DAG commits, then header counters/fill refresh immediately with the same generation as the completion notice.
  #   2. Given unknown coverage or a failed commit, when events reach the reducer, then no fabricated zero coverage or successful compaction badge is shown.
  #   3. Given durable promotion followed by notification failure, when cold restore or explicit reconciliation runs, then the new generation survives without duplicate commit, stale header overwrite or a claim that completion was already delivered.
  #
  # ========================================
  Background: User Story
    As a agent operator
    I want to see committed compaction occupancy and honest coverage immediately
    So that a completion notice and the context header describe the same durable generation

  Scenario: Committed compaction refreshes counters before another provider call
    Given my session header shows a full context before compaction
    When a durable DAG generation commits with a new context snapshot
    Then the background counters and header immediately show that generation's estimated tokens and trigger-relative fill
    And the completion notice identifies the same generation and measured durable coverage

  Scenario: Unknown coverage is not presented as measured zero
    Given a committed compaction has no measured durable coverage
    When its context snapshot reaches my session
    Then the completion notice explicitly reports unknown coverage
    And no rejected or uncommitted operation produces a success badge

  Scenario: Reconciliation does not duplicate completion or restore stale counters
    Given a newer committed generation is already displayed after notification reconciliation
    When an older or duplicate committed snapshot arrives
    Then my header retains the newer generation and counters
    And no duplicate completion notice is delivered

  Scenario: Delayed completion cannot overwrite a newer request context
    Given a measured request context from a committed generation
    When that generation's delayed completion snapshot arrives twice
    Then the measured request fill and current counters remain unchanged
    And the completion notice is delivered exactly once
