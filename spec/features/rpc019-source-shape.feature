@done
@RPC-019
@rust
@source-shape
@tui
@rpc
Feature: RPC-019 source-shape regression for the AgentView MultiLineInput + ScrollbackList port
  """
  RPC-019 pins the source-shape contract so that future refactors of
  the AgentView widget stack cannot silently break the integration
  surface downstream cards depend on.

  Files this card introduces / modifies:

  1. codelet/Cargo.toml — `tui-textarea = "0.7"` workspace dep,
  feature-flagged for crossterm so the Compositor stays on the
  same event-source backbone as `tui-input` and `ratatui`.
  2. codelet/fspec-tui/Cargo.toml — depends on workspace tui-textarea.
  3. codelet/fspec-tui/src/views/agent/multiline_input.rs — new file
  declaring `MultiLineInput` + `InputEventOutcome`. Under 300 LoC.
  4. codelet/fspec-tui/src/views/agent/scrollback.rs — new file
  declaring `ScrollbackList`, `ScrollState`, and re-using the
  existing `RenderedChunk` type from `views/agent.rs`. Under
  300 LoC.
  5. codelet/fspec-tui/src/views/agent.rs — orchestrator swaps
  `input: tui_input::Input` for `input: MultiLineInput` and
  `scrollback: Vec<RenderedChunk>` for `scrollback: ScrollbackList`.
  The `tui_input` import is removed. Stays under 300 LoC.
  6. codelet/fspec-tui/src/components/mod.rs — Action enum gains
  four additive variants `HistoryPrev`, `HistoryNext`,
  `SessionPrev`, `SessionNext`. App::dispatch routing for those
  four variants is deferred to RPC-021 — for RPC-019 they only
  need to exist.

  Existing TS code paths
  (src/tui/components/MultiLineInput.tsx,
  src/tui/components/VirtualList.tsx,
  src/tui/components/ConversationInputArea.tsx) are NOT touched.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the RPC-019 source layout to be locked in by a regression test
    So that future cards inheriting MultiLineInput / ScrollbackList continue to find them where they expect

  Scenario: codelet workspace declares tui-textarea as a dep
    Given codelet/Cargo.toml after RPC-019 lands
    Then the file contains the substring "tui-textarea ="

  Scenario: codelet-fspec-tui declares tui-textarea as a dep
    Given codelet/fspec-tui/Cargo.toml after RPC-019 lands
    Then the file contains the substring "tui-textarea"

  Scenario: New MultiLineInput module exists with the documented surface
    Given the codelet/fspec-tui crate after RPC-019 lands
    Then the file codelet/fspec-tui/src/views/agent/multiline_input.rs exists
    And the file contains the substring "pub struct MultiLineInput"
    And the file contains the substring "pub enum InputEventOutcome"
    And the file contains the substring "Submitted(String)"
    And the file contains the substring "Continued"
    And the file contains the substring "Ignored"

  Scenario: New ScrollbackList module exists with the documented surface
    Given the codelet/fspec-tui crate after RPC-019 lands
    Then the file codelet/fspec-tui/src/views/agent/scrollback.rs exists
    And the file contains the substring "pub struct ScrollbackList"
    And the file contains the substring "pub struct ScrollState"
    And the file contains the substring "pub fn push"
    And the file contains the substring "stick_to_bottom"

  Scenario: AgentView orchestrator now wires the new widgets
    Given codelet/fspec-tui/src/views/agent.rs after RPC-019 lands
    Then the file contains the substring "MultiLineInput"
    And the file contains the substring "ScrollbackList"
    And the file does NOT contain the substring "tui_input::Input"

  Scenario: Action enum gains four navigation variants
    Given codelet/fspec-tui/src/components/mod.rs after RPC-019 lands
    Then the file contains the substring "HistoryPrev"
    And the file contains the substring "HistoryNext"
    And the file contains the substring "SessionPrev"
    And the file contains the substring "SessionNext"

  Scenario: Every file under views/agent/ and views/agent.rs stays under 300 lines
    Given the directory codelet/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator
    When a test counts the line-count of every .rs file
    Then every file in views/agent/ has fewer than 300 lines
    And the orchestrator file views/agent.rs has fewer than 300 lines

  Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
    Given the directory codelet/fspec-tui/src/views/ (including views/agent/) after RPC-019 lands
    When a test scans every *.rs file
    Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
    And no file constructs `tokio::runtime::Builder` or `Runtime::new()`

  Scenario: Existing TS AgentView input + scrollback files are untouched
    Given the project root after RPC-019 lands
    Then the file src/tui/components/MultiLineInput.tsx exists
    And the file src/tui/components/VirtualList.tsx exists
    And the file src/tui/components/ConversationInputArea.tsx exists
