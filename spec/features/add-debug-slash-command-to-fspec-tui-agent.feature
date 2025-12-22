@agent-modal
@tui
@debugging
@AGENT-021
Feature: Add /debug slash command to fspec TUI agent
  """
  NAPI session.rs: Add #[napi] pub fn toggle_debug(&self) -> Result<DebugCommandResultJs> that mirrors CLI repl_loop.rs:36-67 logic
  NAPI types.rs: Add DebugCommandResultJs struct with enabled: bool, sessionFile: Option<String>, message: String
  AgentModal.tsx handleSubmit: Before session.prompt(), check if inputValue.trim() === '/debug' and call session.toggleDebug() instead
  AgentModal.tsx state: Add isDebugEnabled state, update header to show DEBUG indicator when true
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Typing /debug in the input toggles debug capture mode on/off
  #   2. Debug events are written to .codelet/debug-capture-*.json files (same as CLI)
  #   3. Debug mode captures compaction.check, compaction.triggered, token.update, and api.* events
  #   4. A visual indicator shows when debug mode is active (e.g., in header)
  #   5. NAPI session must expose toggleDebug() method that calls handle_debug_command() and sets session metadata
  #   6. AgentModal must intercept /debug input before sending to agent (same pattern as CLI repl_loop.rs:35-68)
  #   7. When debug enabled, session metadata (provider, model, context_window) must be set on the debug capture manager
  #
  # EXAMPLES:
  #   1. User types /debug, sees 'Debug capture enabled' message, header shows DEBUG indicator
  #   2. User types /debug again, sees 'Debug capture disabled' message, DEBUG indicator disappears
  #   3. With debug enabled, user sends prompt, .codelet/debug-capture-*.json is created with api.request, api.response.start, compaction.check events
  #   4. When context approaches 180k tokens with debug on, compaction.triggered event is captured showing threshold exceeded
  #
  # ========================================
  Background: User Story
    As a developer debugging the fspec TUI agent
    I want to enable debug capture mode via /debug command
    So that I can see compaction events and token tracking to diagnose why compaction isn't triggering

  @smoke
  Scenario: Enable debug capture mode
    Given I have the fspec TUI agent modal open
    When I type "/debug" in the input and submit
    Then I should see a message "Debug capture started. Writing to: ~/.codelet/debug/session-*.jsonl"
    And the header should show a DEBUG indicator
    And session metadata should be set on the debug capture manager

  @smoke
  Scenario: Disable debug capture mode
    Given I have the fspec TUI agent modal open
    And debug capture mode is enabled
    When I type "/debug" in the input and submit
    Then I should see a message "Debug capture stopped. Session saved to: ~/.codelet/debug/session-*.jsonl"
    And the DEBUG indicator should disappear from the header

  @integration
  Scenario: Debug events captured during prompt
    Given I have the fspec TUI agent modal open
    And debug capture mode is enabled
    When I send a prompt to the agent
    Then the debug session file should contain "api.request" event
    And the debug session file should contain "api.response.start" event
    And the debug session file should contain "compaction.check" event
    And the debug session file should contain "token.update" event

  @integration
  Scenario: Compaction triggered event captured
    Given I have the fspec TUI agent modal open
    And debug capture mode is enabled
    And the context has accumulated close to 180k tokens
    When the next prompt triggers compaction
    Then the debug session file should contain "compaction.triggered" event
    And the event should show the threshold was exceeded
