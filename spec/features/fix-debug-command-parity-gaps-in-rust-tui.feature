@done
@RPC-430
@rust
@tui
@slash-command
@debug
@session-management
Feature: /debug command parity with TypeScript TUI
  """
  Fixes four critical gaps in the Rust TUI's /debug command compared to the TypeScript TUI:
  1. Debug directory path: ~/.fspec instead of .fspec/debug
  2. Pre-session toggle support
  3. Debug state hydration on session attach
  4. Debug state propagation on session creation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Debug directory MUST resolve to ~/.fspec (user home directory), not .fspec/debug (project-relative). This matches the TypeScript TUI's getFspecUserDir() which returns join(homedir(), '.fspec').
  #   2. /debug MUST support pre-session toggle: when there is no active session, it toggles a global pre-session debug state and emits a scrollback notice. Mirrors TypeScript's toggleDebug(debugDir) path.
  #   3. On session attach (EnterWorkUnit, AttachToSession, SessionCreated), the TUI MUST hydrate debug state by calling backend.get_debug_enabled(session_id) and storing the result in AgentViewStore.debug_enabled_by_session. Mirrors TypeScript's applyPendingDebugState() fallback.
  #   4. On session creation, if the global pre-session debug state is enabled, the TUI MUST propagate debug state to the new session by calling backend.set_debug_enabled(session_id, true). Mirrors TypeScript's AgentView.tsx:1846-1856 path.
  #   5. The debug_enabled_by_session HashMap in AgentViewStore serves as both the live state AND the pending state buffer. When a DebugStateChange stream chunk arrives for any session (active or not), the value is stored in the map.
  #
  # ========================================
  Background: User Story
    As a Rust TUI user
    I want to toggle debug capture with /debug
    So that I have full parity with the TypeScript TUI's debug capture behavior

  # ========================================
  # SCENARIO GROUP: Debug Directory Path
  # ========================================
  Scenario: /debug resolves debug directory to ~/.fspec by default
    Given the HOME environment variable is set to "/home/testuser"
    And the FSPEC_DEBUG_DIR environment variable is NOT set
    When the /debug handler resolves the debug directory
    Then the resolved debug directory path equals "/home/testuser/.fspec"

  Scenario: /debug respects FSPEC_DEBUG_DIR environment variable override
    Given the FSPEC_DEBUG_DIR environment variable is set to "/custom/debug/path"
    When the /debug handler resolves the debug directory
    Then the resolved debug directory path equals "/custom/debug/path"

  # ========================================
  # SCENARIO GROUP: Pre-Session Toggle
  # ========================================
  Scenario: /debug toggles pre-session debug state when no active session exists
    Given an App with NO current session and pre_session_debug_enabled is false
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then App.pre_session_debug_enabled is true
    And the debug directory argument equals the resolved home path "~/.fspec"

  Scenario: /debug toggles pre-session debug state off on second invocation
    Given an App with NO current session and pre_session_debug_enabled is true
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then App.pre_session_debug_enabled is false

  # ========================================
  # SCENARIO GROUP: Debug Hydration on Session Attach
  # ========================================
  Scenario: Session attach hydrates debug state from backend
    Given an App with NO sessions and a MockBackend whose get_debug_enabled returns Ok(true) for session s-1
    When AttachToSession(s-1) is dispatched
    Then within 1 second backend.get_debug_enabled(s-1) is called
    And AgentViewStore.debug_enabled_for(s-1) returns Some(true)

  # ========================================
  # SCENARIO GROUP: Debug Propagation on Session Creation
  # ========================================
  Scenario: Session creation propagates pre-session debug state when enabled
    Given an App with pre_session_debug_enabled set to true
    And a MockBackend whose create_session returns Ok(s-1)
    When SessionCreated(s-1) is dispatched
    Then within 1 second backend.set_debug_enabled(s-1, true) is called
    And AgentViewStore.debug_enabled_for(s-1) returns Some(true)

  Scenario: Session creation does NOT propagate debug state when pre-session is disabled
    Given an App with pre_session_debug_enabled set to false
    And a MockBackend whose create_session returns Ok(s-1)
    When SessionCreated(s-1) is dispatched
    Then backend.set_debug_enabled is NOT called

  # ========================================
  # SCENARIO GROUP: Existing /debug with active session (regression check)
  # ========================================
  Scenario: /debug with active session calls toggle_debug with correct directory
    Given an App with an open session s-1
    And the HOME environment variable is set to "/home/testuser"
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second backend.toggle_debug(s-1, "/home/testuser/.fspec") is called
    And the resolved debug directory path equals "/home/testuser/.fspec"
