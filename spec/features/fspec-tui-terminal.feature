@done
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: Terminal lifecycle + panic-hook idempotency

  TerminalGuard::init() enables alt-screen + raw mode + mouse capture
  + bracketed paste, registers an idempotent panic hook (guarded by
  std::sync::Once) that restores the terminal before delegating to
  the previous panic hook chain, and returns a guard whose Drop runs
  the same restoration.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want TerminalGuard::init() and Drop to enable + restore alt-screen + raw mode + mouse capture + bracketed paste, and the panic hook to restore the terminal even when Drop does not run
    So that a panic mid-render never leaves the user's terminal in raw mode and repeated init calls do not stack hooks

  Scenario: TerminalGuard::init enables alt-screen + raw mode + mouse + bracketed paste
    Given a clean process state (no terminal modes set)
    When `TerminalGuard::init()` returns Ok
    Then crossterm raw mode is enabled
    And the alt-screen has been entered
    And EnableMouseCapture has been written to stdout
    And EnableBracketedPaste has been written to stdout

  Scenario: TerminalGuard::Drop restores the terminal
    Given a TerminalGuard::init()-initialised terminal
    When the TerminalGuard is dropped at end of scope
    Then crossterm raw mode is disabled
    And the alt-screen has been exited
    And DisableMouseCapture has been written to stdout
    And DisableBracketedPaste has been written to stdout

  Scenario: Panic mid-render restores the terminal via the registered panic hook
    Given a TerminalGuard::init() has registered the fspec-tui panic hook
    When test code wraps a `terminal.draw(|_| panic!("boom"))` call in `std::panic::catch_unwind`
    Then the panic is captured
    And `crossterm::terminal::is_raw_mode_enabled()` returns false afterwards
    And the alt-screen has been exited

  Scenario: Panic-hook registration is idempotent
    Given the fspec-tui panic hook has been registered once
    When TerminalGuard::init() is called a second time in the same process
    Then the panic hook is not re-registered
    And the previous panic hook chain is preserved
