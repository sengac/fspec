@BUG-214
@tui
@integration
@compaction
@context-management
@phase-1
Feature: Escape cancels automatic compaction through real session transports
  """
  Architecture notes:
  Exercise a real Escape key event through the Rust TUI event handler and its
  session interrupt cascade, using embedded and local WebSocket transports.
  Reuse the BUG-210 session and transport fixture without altering its setup.
  The pending builder must belong to pre-prompt automatic compaction reached
  through run_agent_stream, not already-cancellable terminal-overflow recovery.
  Cancellation must stop the pending operation without a provider response,
  deadline expiry or forceful task abortion. Use deterministic local fixtures,
  with no live provider credentials or external network dependencies.
  Existing dialog, popup, selection and idle Escape priorities remain unchanged;
  these scenarios have no higher-priority transient UI consuming the keypress.
  Footer presentation and status schema remain owned by BUG-213.
  """

  Background: User Story
    As a developer using the Rust terminal interface
    I want Escape to stop automatic compaction in my active session
    So that I can interrupt context processing without losing my conversation

  Scenario: Escape cancels pending pre-prompt compaction through the embedded transport
    Given the TUI uses the embedded session transport
    And the focused session has a pending pre-prompt automatic compaction builder
    And no dialog, popup or selection consumes Escape
    When I press Escape through the TUI event handler
    Then the focused session receives the interrupt and its pending compaction stops promptly
    And no successful compaction or continuation inference occurs
    And the original conversation remains unchanged and accessible
    And compaction activity is cleared for that session without affecting another session

  Scenario: Escape cancels pending pre-prompt compaction through the WebSocket transport
    Given the TUI uses a local WebSocket session transport
    And the focused session has a pending pre-prompt automatic compaction builder
    And no dialog, popup or selection consumes Escape
    When I press Escape through the TUI event handler
    Then the focused session receives the interrupt and its pending compaction stops promptly
    And no successful compaction or continuation inference occurs
    And the original conversation remains unchanged and accessible
    And compaction activity is cleared for that session without affecting another session
