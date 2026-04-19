@BUG-133
Feature: [DEBUG] badge in SessionHeader is not session-aware like [ISOLATED]

  """
  Reference pattern (isIsolated): Rust emits StreamChunk::IsolationStateChange (codelet/napi/src/types.rs:543, :718) -> globalSessionStreamManager.ts:292-309 listener dispatches setIsolationState -> sessionStore.ts:37 (state), :184 (setter), :254 (useIsIsolated selector) -> AgentView reads via useIsIsolated() (AgentView.tsx:909). This is the exact Rust-authoritative, event-driven, per-session pattern [DEBUG] must follow.
  Rust side: Per-session field BackgroundSession::is_debug_enabled: AtomicBool exists (session_manager.rs:527, :669, :805, :810) with NAPI accessors session_get_debug_enabled/session_set_debug_enabled (:6994, :7001). The /debug handler calls session_toggle_debug (:7555) which flips the global DebugCaptureManager (codelet/common/src/debug_capture/mod.rs:25 OnceLock singleton, :89 handle_debug_command_with_dir) and mirrors result into the per-session AtomicBool (:7563). NO StreamChunk variant exists for debug state change (grep for DebugToggled/DebugStateChange in codelet/napi/src/types.rs returns 0 matches).
  TS side gaps: sessionStore.ts has NO debug state/setter/selector (only logger.debug calls). useRustDebugEnabled(sessionId) hook exists (useRustSessionState.ts:313) but is unused by AgentView/SessionHeader. /debug command handled in AgentView.tsx:1611-1639 calls sessionToggleDebug then setIsDebugEnabled (local useState, not Zustand). A duplicate, unreachable handler exists at AgentView.tsx:2664-2689 (dead code). No refreshRustState or stream event emission triggers Zustand update.
  TUI LAYER 1 - Add Zustand state to sessionStore.ts. Mirror the isIsolated pattern: (a) Add isDebugEnabled: boolean to per-session state shape (~line 37 near isIsolated), default false. (b) Add setDebugState(sessionId, enabled) action (~line 184 near setIsolationState). (c) Add useIsDebugEnabled() selector hook (~line 254 near useIsIsolated) that reads from the current session's state.
  TUI LAYER 2 - Add stream listener in globalSessionStreamManager.ts. Handle the new DebugStateChange chunk (~line 292-309 near the IsolationStateChange handler). When chunk type is DebugStateChange, call sessionStore.getState().setDebugState(sessionId, chunk.enabled). This is the event replication path from Rust to Zustand.
  TUI LAYER 3 - Rewire AgentView.tsx and SessionHeader.tsx. (a) Remove the local useState(isDebugEnabled) at AgentView.tsx:843. (b) Remove the derived displayIsDebugEnabled OR logic at AgentView.tsx:1259. (c) Remove the duplicate /debug handler at AgentView.tsx:2664-2689 (dead code). (d) In the primary /debug handler (AgentView.tsx:1611-1639) keep the sessionToggleDebug call but remove setIsDebugEnabled - the Zustand store will be updated by the stream event. (e) Replace isDebugEnabled={displayIsDebugEnabled} prop at AgentView.tsx:5228 with isDebugEnabled={isDebugEnabled} sourced from useIsDebugEnabled() selector. (f) SessionHeader.tsx itself needs no changes (it just reads the prop).
  TUI LAYER 4 - Remove unused code. (a) The useRustDebugEnabled(sessionId) hook in useRustSessionState.ts:313 is currently unused - evaluate whether to remove it entirely since Zustand now owns the state, or keep it as a fallback for initial hydration when a session is first attached. (b) The isDebugEnabled field in rustSnapshot (useRustSessionState.ts:71, :91, :183) can be removed from the snapshot shape since Zustand is now authoritative.
  TUI LAYER 5 - Initial hydration on session switch. When the user switches to a session (or first attaches), the Zustand store must be seeded with that session's current debug state from Rust. Use sessionGetDebugEnabled(sessionId) at attach time and call setDebugState(sessionId, result) so the badge renders correctly before any stream events arrive. Mirror how isIsolated is hydrated on session creation/attach.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. [DEBUG] badge state MUST be sourced from a Zustand selector that reflects per-session Rust state (mirroring useIsIsolated)
  #   2. Rust must emit a StreamChunk::DebugStateChange { session_id, enabled } event when session debug state toggles
  #   3. globalSessionStreamManager must listen for DebugStateChange and dispatch into sessionStore per session id
  #   4. The local useState(isDebugEnabled) in AgentView.tsx:843 and the OR at AgentView.tsx:1259 must be removed; the duplicate /debug handler at AgentView.tsx:2664-2689 must also be removed
  #
  # EXAMPLES:
  #   1. Switching between two sessions where only session A has /debug enabled: [DEBUG] badge must appear only while session A is active, and disappear when switching to session B
  #   2. Toggling /debug in session A must fire a Rust stream event that updates Zustand so the header re-renders without relying on local React state
  #
  # QUESTIONS (ANSWERED):
  #   Q: The underlying DebugCaptureManager is a process-wide OnceLock singleton writing to a single shared JSONL file. Should per-session [DEBUG] badge semantics mean 'this session toggled debug on' (bookkeeping only), or should capture be refactored to be truly per-session? Matching the isIsolated pattern treats the badge as per-session state mirroring the AtomicBool.
  #   A: Refactor to truly per-session capture. Debug log data should be from ONE session, not mixed. BUG-133 handles the TUI wiring layer; a new story (BUG-134) handles the underlying Rust architectural refactor to make DebugCaptureManager per-session.
  #
  # ASSUMPTIONS:
  #   1. Refactor to truly per-session capture. Debug log data should be from ONE session, not mixed. BUG-133 handles the TUI wiring layer; a new story (BUG-134) handles the underlying Rust architectural refactor to make DebugCaptureManager per-session.
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to fix the [DEBUG] badge to reflect only the current session's debug state, sourced from Rust via stream events into Zustand
    So that the badge accurately represents per-session debug capture state, consistent with other session-aware badges like [ISOLATED]

  @session-switch
  Scenario: DEBUG badge reflects only the active session's debug state when switching sessions
    Given session A has debug capture enabled
    And session B has debug capture disabled
    When I switch to session A
    Then the SessionHeader should display the "[DEBUG]" badge
    When I switch to session B
    Then the SessionHeader should not display the "[DEBUG]" badge

  @stream-event
  Scenario: Toggling debug fires a Rust stream event that updates Zustand store
    Given session A is active with debug capture disabled
    When I run the "/debug" command in session A
    Then Rust should emit a DebugStateChange stream event with enabled true for session A
    And the Zustand sessionStore should contain isDebugEnabled true for session A
    And the SessionHeader should display the "[DEBUG]" badge

  @stream-event
  Scenario: Disabling debug fires a stream event that removes the badge
    Given session A is active with debug capture enabled
    When I run the "/debug" command in session A
    Then Rust should emit a DebugStateChange stream event with enabled false for session A
    And the Zustand sessionStore should contain isDebugEnabled false for session A
    And the SessionHeader should not display the "[DEBUG]" badge

  @hydration
  Scenario: Debug state is hydrated from Rust when attaching to an existing session
    Given session A previously had debug capture enabled
    And session A is not currently attached in the TUI
    When I attach to session A
    Then the Zustand sessionStore should be seeded with isDebugEnabled true for session A
    And the SessionHeader should display the "[DEBUG]" badge

  @cleanup
  Scenario: Local React debug state and duplicate handler are removed
    Given the AgentView component is rendered
    Then there should be no local useState for isDebugEnabled in AgentView
    And the isDebugEnabled prop to SessionHeader should be sourced from a Zustand useIsDebugEnabled selector
    And there should be no duplicate "/debug" handler in AgentView
