@done
@bug-fix
@compaction
@session-management
@tui
@CMPCT-033
Feature: Compaction SessionStateChange drops sessionId — wrong session shows Compacting indicator
  """
  Rust NAPI already carries session_id via GlobalChunkCallbackArgs at codelet/napi/src/session_manager.rs:67-70 and :971-972 — no ABI change needed
  Change SessionChunkHandler type signature in src/tui/services/globalSessionStreamManager.ts:23 from (chunk) => void to (sessionId, chunk) => void
  Update useSessionStreamManager to pass routed sessionId through to the persistentChunkHandler registered in AgentView
  Fix src/tui/handlers/persistentSessionStateHandler.ts:57-62 — accept sessionId parameter, use it instead of deps.getCurrentSessionId() for Compacting state
  Fix src/tui/components/AgentView.tsx:2352-2366 — use the routed sessionId from the handler signature instead of activeSessionId for the Compacting branch
  Fix src/tui/components/AgentView.tsx:3394-3405 — use the routed sessionId instead of currentSessionIdRef.current for the Compacting branch
  Reference pattern: IsolationStateChange handler at src/tui/services/globalSessionStreamManager.ts:288-308 and FooterStateUpdate at :313-325 already correctly use routed sessionId — mirror this pattern for Compacting
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionChunkHandler type must propagate the routed sessionId from the outer NAPI callback rather than dropping it
  #   2. Compaction state consumers must use the routed sessionId (from the chunk stream) rather than currentSessionIdRef.current or activeSessionId
  #   3. The fix must follow the existing IsolationStateChange/FooterStateUpdate outer-routing pattern (no NAPI ABI change needed)
  #   4. All three compaction consumers must be fixed: persistentSessionStateHandler.ts, AgentView.tsx line 2352-2366, AgentView.tsx line 3394-3405
  #
  # EXAMPLES:
  #   1. Session A is being viewed; Session B (background) auto-compacts via hook — the Compacting indicator must appear on Session B's UI slot, not Session A's
  #   2. User has two sessions open; background session reaches context limit and triggers auto-compaction — the Compacting badge appears on that background session's row, not on the foreground session
  #   3. User runs /compact manually on session B while viewing session A — Compacting indicator shows on session B (not session A)
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to have the Compacting status indicator attach to the actual session being compacted
    So that background/non-active sessions that auto-compact show their Compacting status correctly rather than being attributed to whichever session the user is currently viewing

  Scenario: Background session auto-compacts while a different session is viewed
    Given I am a fspec developer with Session A in the foreground view
    And Session B is running in the background
    When Session B auto-compacts via a hook-triggered context-limit event
    Then the Compacting indicator appears on Session B's UI slot
    And the Compacting indicator does not appear on Session A's UI slot

  Scenario: Two sessions open and the background one triggers auto-compaction at its context limit
    Given I am a fspec developer with two sessions open
    And Session A is the foreground session
    And Session B is the background session
    When Session B reaches its context limit and triggers auto-compaction
    Then the Compacting badge appears on Session B's row
    And the Compacting badge does not appear on Session A's row

  Scenario: Manual /compact on a non-active session
    Given I am a fspec developer viewing Session A
    And Session B is also open but not currently active
    When I run /compact manually targeting Session B
    Then the Compacting indicator shows on Session B
    And the Compacting indicator does not show on Session A

  Scenario: SessionChunkHandler propagates routed sessionId
    Given the NAPI stream callback delivers a SessionStateChange chunk with state "Compacting" for session "sess-B"
    And I am currently viewing session "sess-A"
    When the TUI routes the chunk through the SessionChunkHandler
    Then the handler receives both the routed sessionId "sess-B" and the chunk
    And the Compacting state is attributed to session "sess-B"
    And the Compacting state is not attributed to session "sess-A"
