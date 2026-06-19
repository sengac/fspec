@done
@RPC-095 @rpc @input @agent-view @critical
Feature: AgentView Esc Cascade Input Clear

  """
  RPC-095 — Esc-cascade level 6 (input clear) extension to the RPC-051 cascade.

  TS reference: src/tui/components/AgentView.tsx:4731-4773
  Rust implementation: codelet/fspec-tui/src/app/dispatch_esc_cascade.rs

  Adds two new branches BEFORE the existing L5 BackToBoard fallback:
  - L6: input.trim() non-empty → clear input, stay on AgentView
  - L7 (deferred to follow-up): exit confirmation dialog
  """

  Background: User Story
    As a fspec user running the Rust ratatui AgentView
    I want pressing Esc on a non-empty input to clear the text rather than discarding it by navigating away
    So that I don't accidentally lose typed text when trying to dismiss the view

  Scenario: Esc when idle with non-empty input clears the buffer
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Idle
    And the input buffer contains the text "hello world"
    When the user presses Esc
    Then the MultiLineInput value equals ""
    And Navigator.active_view stays at ViewMode::Agent
    And backend.interrupt is NEVER called

  Scenario: Esc when idle with whitespace-only input is treated as empty
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Idle
    And the input buffer contains only whitespace "   "
    When the user presses Esc
    Then Action::BackToBoard is dispatched
    And Navigator.active_view becomes ViewMode::Board
