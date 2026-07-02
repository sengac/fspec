@done
@validator
@validation
@regression
@RPC-074
@rust
@source-shape
@tui
Feature: RPC-074 source-shape regression — divergent /clear strings absent
  """
  RPC-074 source-shape regression test. The TS-divergent strings
  `[notice] /clear: history cleared`, `[error] /clear failed: <e>`, and
  the `UserNotification("history cleared")` chunk broadcast that
  RPC-046 / RPC-037 originally introduced are pure Rust-side invention
  with no counterpart in the TS reference at
  src/tui/components/AgentView.tsx:1554-1564 (handleClearCommand). This
  feature locks down the absence of those literals at the Rust source
  level so they cannot creep back in.
  """

  Background: User Story
    As a Rust port maintainer
    I want grep-level source-shape assertions that ban the TS-divergent /clear strings
    So that no future card can quietly reintroduce them and break TS parity

  Scenario: Rust source files do not contain TS-divergent /clear strings
    Given the file codelet/fspec-tui/src/app/dispatch_slash_clear.rs is read into memory
    And the file codelet/core/src/session_manager_handle.rs is read into memory
    When the test searches both files for the literal strings "history cleared" and "[notice] /clear"
    Then dispatch_slash_clear.rs contains neither literal
    And session_manager_handle.rs does not contain the literal string "\"history cleared\""
