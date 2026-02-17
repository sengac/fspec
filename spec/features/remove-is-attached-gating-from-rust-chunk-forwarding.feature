@BRIDGE-012
Feature: Remove is_attached gating from Rust chunk forwarding

  """
  session_id is already properly routed for tool callbacks (e.g., FspecCommandRequest) - captured in TS closure and passed to sessionSendFspecResult
  PROBLEM: is_attached gating in handle_output() drops chunks BEFORE they reach TypeScript. watcher_broadcast bypasses this (so Telegram works), but attached_callback is gated (so TUI doesn't).
  SOLUTION: Remove is_attached check, always call callback. Or better: one global callback that receives (session_id, chunk) for ALL sessions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust exposes a single global callback that TypeScript registers once at startup. This callback receives (session_id, chunk) for ALL chunks from ALL sessions. Remove per-session attach()/detach() pattern entirely.
  #   2. TypeScript is responsible for routing and displaying chunks based on session_id - Rust has no knowledge of which session is "active"
  #
  # EXAMPLES:
  #   1. When bridge sends input, the TUI shows both the bridge input AND the LLM response chunks in the conversation
  #   2. When user types in TUI, the LLM response chunks appear in the conversation
  #
  # ========================================

  Background: User Story
    As a developer
    I want to receive all session chunks in the TUI regardless of input source
    So that bridge/watcher/user inputs all display correctly

  # ==========================================================================
  # Scenario: Bridge input displays response in TUI
  # Rule: Rust exposes a single global callback that TypeScript registers once
  #       at startup. This callback receives (session_id, chunk) for ALL chunks
  #       from ALL sessions.
  # ==========================================================================
  Scenario: Bridge input displays both input and response in TUI
    Given a session is active with the global chunk callback registered
    And a Telegram bridge is connected to the session
    When the bridge sends input to the session
    Then the TUI should display the bridge input in the conversation
    And the TUI should display the LLM response chunks in the conversation

  # ==========================================================================
  # Scenario: Keyboard input displays response in TUI
  # Rule: TypeScript is responsible for routing and displaying chunks based on
  #       session_id - Rust has no knowledge of which session is "active"
  # ==========================================================================
  Scenario: Keyboard input displays response in TUI
    Given a session is active with the global chunk callback registered
    When the user types input directly in the TUI
    Then the TUI should display the LLM response chunks in the conversation
