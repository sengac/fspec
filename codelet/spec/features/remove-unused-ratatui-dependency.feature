@REFAC-008
Feature: Remove unused ratatui dependency from TUI crate

  """
  All output uses println! with ANSI escape codes, not ratatui rendering
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ratatui must be removed from all Cargo.toml files (workspace, tui, cli)
  #   2. terminal.rs must be simplified to remove ratatui imports and unused TuiTerminal type
  #   3. All existing tests must continue to pass after changes
  #   4. crossterm functionality (raw mode, event stream, keyboard enhancements) must be preserved
  #
  # EXAMPLES:
  #   1. Before: tui/Cargo.toml contains 'ratatui.workspace = true'. After: line removed
  #   2. Before: terminal.rs imports ratatui::Terminal. After: imports removed, TuiTerminal type deleted
  #   3. cargo test runs successfully after all changes
  #   4. cargo clippy -- -D warnings passes after all changes
  #
  # ========================================

  Background: User Story
    As a developer maintaining codelet
    I want to remove the unused ratatui dependency
    So that I reduce binary size and compile time without functional changes

  Scenario: Remove ratatui from workspace Cargo.toml
    Given the workspace Cargo.toml contains 'ratatui = "0.29"'
    When I remove the ratatui dependency line
    Then the workspace Cargo.toml no longer contains ratatui


  Scenario: Remove ratatui from tui crate Cargo.toml
    Given the tui/Cargo.toml contains 'ratatui.workspace = true'
    When I remove the ratatui dependency line
    Then the tui/Cargo.toml no longer contains ratatui


  Scenario: Remove ratatui from cli crate Cargo.toml
    Given the cli/Cargo.toml contains 'ratatui.workspace = true'
    When I remove the ratatui dependency line
    Then the cli/Cargo.toml no longer contains ratatui


  Scenario: Simplify terminal.rs to remove unused TuiTerminal type
    Given terminal.rs contains 'use ratatui' and 'pub type TuiTerminal'
    When I remove the ratatui imports and TuiTerminal type
    Then terminal.rs no longer imports ratatui
    And the TuiTerminal type alias is removed
    And the init_terminal function is removed or simplified


  Scenario: All existing tests pass after changes
    Given all ratatui dependencies have been removed
    When I run 'cargo test'
    Then all tests pass
    And terminal.rs has been simplified
    And 'cargo clippy -- -D warnings' passes

