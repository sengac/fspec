@done
@zustand
@bug-fix
@session-management
@header
@tui
@BUG-135
Feature: [DEBUG] badge disappears when cycling back to a previously-visited session via Shift+Left/Right

  """
  Regression introduced by BUG-133. Full root-cause analysis and design proposal in spec/attachments/BUG-135/design-analysis.md
  Zustand sessionStore: replace flat `isDebugEnabled: boolean` field with `debugStateBySession: Map<string, boolean>`. Update setDebugState to take (sessionId, enabled). Remove the `state.isDebugEnabled = false` reset in activateSession (sessionStore.ts:157). Remove the isDebugEnabled=false reset in clearAndResetSession (sessionStore.ts:114).
  useIsDebugEnabled selector: read `state.debugStateBySession.get(state.currentSessionId) ?? false`. Pure Zustand read, no NAPI calls per render.
  globalSessionStreamManager: DebugStateChange handler (line ~355) must always call setDebugState(sessionId, enabled) regardless of whether sessionId equals currentSessionId — this populates the map for all sessions. The pendingDebugState map becomes unnecessary (since setDebugState now addresses any session directly) and can be removed or kept for back-compat. applyPendingDebugState remains but simply invokes setDebugState with Rust's ground truth.
  AgentView session-attach paths: in resumeSessionById (AgentView.tsx:3633) and peer attach sites (1731, 3735, 3956, 4278), after applyPendingDebugState, also call useSessionStore.getState().setDebugState(sessionId, sessionGetDebugEnabled(sessionId)) as the ground-truth fallback from Rust's per-session AtomicBool.
  SessionHeader.tsx requires no changes — the badge rendering at line 169-171 is correct; only its upstream state pipeline is broken.
  Reference pattern divergence: isIsolated uses a flat boolean + reset-on-activate because isolation is immutable per session. Debug state is toggleable at runtime and therefore requires a per-session Map. Do NOT blindly copy the isIsolated pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Zustand must store debug state as a per-session Map<sessionId, boolean>, not a flat boolean
  #   2. activateSession MUST NOT reset debug state; each session retains its own entry across switches
  #   3. setDebugState action MUST accept (sessionId, enabled) and write into debugStateBySession map
  #   4. useIsDebugEnabled selector MUST read debugStateBySession.get(currentSessionId) and return false if absent
  #   5. On session attach/resume, after applyPendingDebugState, Zustand MUST be re-seeded from Rust's sessionGetDebugEnabled(sessionId) as a hydration fallback
  #   6. SessionHeader.tsx itself MUST NOT be modified — the fix lives entirely in sessionStore, globalSessionStreamManager, and AgentView attach paths
  #
  # EXAMPLES:
  #   1. User enables /debug on session A, switches to session B (badge disappears), switches back to A — the [DEBUG] badge reappears
  #   2. User enables /debug on both sessions A and B, then disables /debug on A — session B still shows [DEBUG], session A no longer shows it
  #   3. User cycles Shift+Right through A→B→C→B→A without toggling debug — each session shows exactly the badge state it had when last toggled
  #   4. User attaches to a session that already had debug capture enabled before the TUI was opened — the [DEBUG] badge is visible immediately on attach
  #   5. While user is viewing session B, a debug toggle for session A (via a watcher or IPC) updates A's state silently — switching to A then shows the correct badge without any additional refresh
  #
  # ========================================

  Background: User Story
    As a TUI user running multiple concurrent sessions
    I want to switch back and forth between sessions via Shift+Left/Right without losing any session's debug state
    So that the [DEBUG] badge accurately reflects each session's debug capture state regardless of how often I switch

  @session-switch @regression
  Scenario: DEBUG badge reappears when cycling back to a session that had debug enabled
    Given session A has debug capture enabled
    And session B has debug capture disabled
    And I am currently viewing session A with the [DEBUG] badge visible
    When I press Shift+Right to switch to session B
    Then the [DEBUG] badge should not be visible
    When I press Shift+Left to switch back to session A
    Then the [DEBUG] badge should be visible again

  @session-switch @regression
  Scenario: Each session retains its own debug state across multiple switches
    Given session A has debug capture enabled
    And session B has debug capture disabled
    And session C has debug capture enabled
    When I cycle through sessions A, B, C, B, A using Shift+Right and Shift+Left
    Then session A should always show the [DEBUG] badge when active
    And session B should never show the [DEBUG] badge when active
    And session C should always show the [DEBUG] badge when active

  @state-management
  Scenario: Toggling debug on one session does not affect other sessions' state
    Given session A has debug capture enabled
    And session B has debug capture enabled
    When I run the "/debug" command in session A to disable debug capture
    Then session A should not show the [DEBUG] badge
    When I press Shift+Right to switch to session B
    Then session B should still show the [DEBUG] badge

  @hydration
  Scenario: DEBUG badge appears when attaching to a session that already has debug enabled in Rust
    Given session A exists in Rust with is_debug_enabled true
    And the TUI has no pending debug state for session A
    When I attach to session A in the TUI
    Then the Zustand store should be seeded with debug enabled for session A
    And the [DEBUG] badge should be visible

  @stream-event
  Scenario: Debug stream event for an inactive session updates its stored state
    Given I am currently viewing session B
    And session A has debug capture disabled
    When Rust emits a DebugStateChange event with enabled true for session A
    Then the Zustand store should record debug enabled true for session A
    And the [DEBUG] badge should not be visible on session B
    When I press Shift+Left to switch to session A
    Then the [DEBUG] badge should be visible without any additional refresh
