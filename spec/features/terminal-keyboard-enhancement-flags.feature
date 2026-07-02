@done
@rust
@tui
@agent-view
@RPC-402
Feature: Terminal keyboard enhancement flags push and pop lifecycle
  """
  Seam: terminal.rs exposes ModeCommand enum + TerminalModePlan { setup, teardown } +
  terminal_mode_plan(enhancement_supported: bool). enable_terminal_modes/restore paths are driven
  off the same plan so tests pin real behavior. Gate flag push on crossterm
  supports_keyboard_enhancement() called AFTER raw mode is enabled; Pop issued in teardown BEFORE
  LeaveAlternateScreen; teardown (incl. Drop/panic restore) must be best-effort and never fail init.
  """

  Background: User Story
    As a TUI user composing a message in the agent view
    I want the terminal put into keyboard-enhancement mode when my terminal supports it
    So that modified Enter keys like Shift+Enter are distinguishable and multi-line input works

  Scenario: Terminal without keyboard-enhancement support skips flag push and pop
    Given the terminal does not support keyboard enhancement flags
    When the terminal modes are initialized
    Then keyboard enhancement flags are not pushed
    And teardown does not issue a pop of keyboard enhancement flags

  Scenario: Supporting terminal pushes flags on init and pops them before leaving the alternate screen
    Given the terminal supports keyboard enhancement flags
    When the terminal modes are initialized and then torn down
    Then keyboard enhancement flags are pushed after raw mode is enabled
    And a pop of keyboard enhancement flags is issued before leaving the alternate screen
