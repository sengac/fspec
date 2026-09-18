@CMPCT-049
@agent-core
@logging
@context-management
@tui
Feature: Compaction diagnostic log-level hygiene — routine propagation at DEBUG, lifecycle at INFO
  """
  New/changed code inventory: (1) rust/agent-loop/src/agent_loop.rs — the per-turn "[compaction-status] turn START" / "turn END — checking for pending DAG" / "turn END — compaction flag was set" / "turn END — watchdog retry pending" lines drop to DEBUG; (2) rust/agent-loop/src/background_output.rs — the routine "Done: no pending compaction" line drops to DEBUG while the rare "compaction/pending-DAG active" line stays INFO; (3) rust/sessions/src/background_session.rs — set_compaction_progress / update_compaction_progress drop to DEBUG, set_status transitions stay INFO; (4) rust/fspec-tui/src/app/bootstrap.rs — the TUI status-recv line drops to DEBUG; (5) rust/fspec-tui/src/app/dispatch_stream_chunks.rs — both "TUI store updated" lines (push channel + SessionStateChange chunk) drop to DEBUG; (6) rust/fspec-tui/src/views/agent/animation.rs — the display-mode flip line stays INFO (the single thinking-vs-compacting decision point).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The compaction-status diagnostic markers were added to debug the thinking-vs-compacting indicator. They must remain visible at the default log level for LIFECYCLE decisions (status transitions, display-mode flips, the rare compaction-active Done branch) and silent at DEBUG for routine per-event propagation (channel hops, progress ticks, per-turn bookkeeping) so a normal session does not flood the log with ~4 lines per status change.
  #   2. set_status transitions (from → to) are lifecycle: INFO. set_compaction_progress / update_compaction_progress are per-event progress: DEBUG.
  #   3. TUI store updates (push channel + SessionStateChange chunk arms) and the bootstrap broadcast-recv are channel propagation: DEBUG.
  #   4. The TUI display-mode flip (thinking / compacting / idle) is the indicator decision point: INFO, emitted only on change.
  #   5. The per-turn agent-loop lines (turn START status, turn END pending-DAG check, flag-clear, watchdog-retry-pending) and the routine Done arm ("no pending compaction") are per-turn bookkeeping: DEBUG; the rare Done arm branch ("compaction/pending-DAG active") is a lifecycle deviation: INFO.
  #   6. [generate-compaction] handler lifecycle lines (ENTER, sub-agent launch/complete, pin, fallback, SLOW-lock) stay INFO — compaction is rare and these are the audit trail for the sub-agent run.
  #
  # ========================================

  Background: User Story
    As a developer debugging the compaction status indicator
    I want routine per-event propagation logged at DEBUG and lifecycle decisions at INFO
    So that a normal session does not spam the log while the thinking-vs-compacting trace stays visible

  @logging
  @source-shape
  @CMPCT-049
  Scenario: Routine propagation lines are logged at DEBUG
    Given the compaction diagnostic markers exist in the agent loop, background output, session, and TUI layers
    When the routine per-event lines are emitted
    Then the agent-loop turn START and turn END check lines are tracing::debug!
    And the background_output routine Done arm ("no pending compaction") is tracing::debug!
    And set_compaction_progress and update_compaction_progress are tracing::debug!
    And the TUI bootstrap status-recv and both TUI store-updated lines are tracing::debug!

  @logging
  @source-shape
  @CMPCT-049
  Scenario: Lifecycle decision lines stay at INFO
    Given the compaction diagnostic markers exist in the agent loop, session, and TUI layers
    When the lifecycle decisions are emitted
    Then the set_status transition line is tracing::info!
    And the TUI display-mode flip line is tracing::info!
    And the rare background_output Done arm branch ("compaction/pending-DAG active") is tracing::info!
