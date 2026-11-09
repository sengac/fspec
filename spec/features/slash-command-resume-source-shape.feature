@done
@RPC-049
@session-management
@rust
@tui
@source-shape
Feature: /resume source-shape regression
  """
  RPC-049 split-out feature file. Pins the file-layout invariants for
  the new resume_session wiring:
  * No file under `rust/fspec-tui/src/` references `codelet_napi`
  (RPC-002 invariant — the AgentView never touches NAPI).
  * Every file under the hot-path source directories
  (`rust/fspec-tui/src/app/`,
  `rust/fspec-tui/src/views/agent/`,
  `rust/fspec-tui/src/store/agent_view/`)
  is strictly less than 300 lines of code (the source-shape
  ceiling pinned by RPC-024 / RPC-025 / RPC-026; historical
  infrastructure files outside these directories — e.g.
  `transport/*`, `components/mod.rs`, `compositor_tests.rs` —
  pre-date the ceiling and are not in scope for RPC-049).
  * `rust/fspec-tui/src/app/dispatch.rs` is strictly less than
  300 lines of code.
  * The Action enum gains a `SessionResumeComplete(SessionId)` variant.
  * The dispatch_resume_search_views helper file declares
  `handle_session_resume_complete`.
  TUI-132 (2026-11-09): Action::SessionResumeComplete and handle_session_resume_complete are removed: nothing emitted the action.
  """

  Background: User Story
    As a fspec engineer maintaining the Rust ratatui frontend
    I want the source-shape invariants pinned by an automated regression test
    So that future RPC slices cannot silently regress the 300-LoC ceiling or sneak a codelet-napi reference into fspec-tui

  Scenario: No codelet_napi reference and the 300-LoC ceiling holds
    Given the rust/fspec-tui/src/ tree after the RPC-049 changes
    Then no file under rust/fspec-tui/src/ matches "codelet_napi"
    And every file under rust/fspec-tui/src/app/, rust/fspec-tui/src/views/agent/, and rust/fspec-tui/src/store/agent_view/ is strictly less than 300 lines of code
    And rust/fspec-tui/src/app/dispatch.rs is strictly less than 300 lines of code
