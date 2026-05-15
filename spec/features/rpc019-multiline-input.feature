@done
@RPC-019
@rust
@tui
@ui
@agent-view
@input
@ui-enhancement
Feature: RPC-019 AgentView multi-line input (tui-textarea-backed MultiLineInput)

  """
  RPC-019 (input slice) — AgentView's single-line `tui_input::Input`
  is replaced by a tui-textarea-backed `MultiLineInput` widget.

  Behaviour ported from src/tui/components/MultiLineInput.tsx:
    - Plain Enter submits the buffer (emits Action::InputSubmitted)
      and resets the textarea.
    - Shift+Enter inserts a literal newline.
    - Pasted text with embedded '\n' becomes multi-line in-place.
    - Up/Down arrows move between visual lines while the cursor has a
      neighbour line; on the top/bottom boundary they are forwarded
      Ignored.
    - Shift+arrow chords are translated into navigation Actions
      (HistoryPrev, HistoryNext, SessionPrev, SessionNext) — RPC-019
      ONLY emits them; RPC-021 will route them through App::dispatch.
    - ESC always emits Action::BackToBoard (preserved from RPC-012).
    - When the buffer is empty, the input row paints
      "> Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions
      | 'Tab' select turn)" in dim grey + green prompt.

  Pair: render tests live in
  codelet/fspec-tui/tests/view_agent_multiline_input_rpc019.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a tui-textarea-backed MultiLineInput in AgentView with Shift+Enter newlines, plain-Enter submits, and Shift+arrow forwards
    So that multi-line composition (paste with newlines, Shift+Enter) works the same way the Ink TS AgentView does

  Scenario: Plain Enter on the multi-line input submits and resets the buffer
    Given an AgentView with an empty MultiLineInput
    When the user types "hello world" then presses plain Enter
    Then AgentView emits Action::InputSubmitted("hello world")
    And the MultiLineInput's buffer is empty after submit
    And the MultiLineInput's visible-row count is 1

  Scenario: Shift+Enter inserts a literal newline instead of submitting
    Given an AgentView with an empty MultiLineInput
    When the user types "hello" then presses Shift+Enter then types "world"
    Then no Action::InputSubmitted is emitted yet
    And the MultiLineInput's buffer is exactly "hello\nworld"
    And the MultiLineInput's visible-row count is 2

  Scenario: Plain Enter submits a multi-line buffer with embedded newlines
    Given an AgentView whose MultiLineInput contains "hello\nworld"
    When the user presses plain Enter
    Then AgentView emits Action::InputSubmitted("hello\nworld")
    And the MultiLineInput's buffer is empty after submit

  Scenario: Pasted text with embedded newlines preserves them in the buffer
    Given an AgentView with an empty MultiLineInput
    When the MultiLineInput is fed the bracketed-paste payload "line-a\nline-b\nline-c"
    Then the MultiLineInput's buffer is exactly "line-a\nline-b\nline-c"
    And the MultiLineInput's visible-row count is 3

  Scenario: MultiLineInput auto-grows up to its max visible rows cap of 6
    Given an AgentView with MultiLineInput max_visible_rows = 6
    When the user inserts 8 newlines back-to-back
    Then the MultiLineInput's visible-row count is exactly 6
    And the MultiLineInput's logical line count is 9

  Scenario: Empty MultiLineInput paints the dim placeholder hint with a green > prefix
    Given an AgentView whose MultiLineInput is empty
    When the App renders AgentView against a 100x12 TestBackend
    Then the rendered buffer's input row contains the substring "> Type a message..."
    And the rendered buffer's input row contains the substring "'Shift+↑/↓' history"
    And the rendered buffer's input row contains the substring "'Shift+←/→' sessions"
    And the rendered buffer's input row contains the substring "'Tab' select turn"

  Scenario: Non-empty MultiLineInput hides the placeholder hint
    Given an AgentView whose MultiLineInput contains "draft"
    When the App renders AgentView against a 100x12 TestBackend
    Then the rendered buffer's input area contains the substring "draft"
    And the rendered buffer does NOT contain the substring "Type a message..."

  Scenario: Shift+Up emits Action::HistoryPrev without modifying the buffer
    Given an AgentView whose MultiLineInput contains "draft"
    When the user presses Shift+Up
    Then AgentView emits Action::HistoryPrev
    And the MultiLineInput's buffer is still exactly "draft"

  Scenario: Shift+Down emits Action::HistoryNext without modifying the buffer
    Given an AgentView whose MultiLineInput contains "draft"
    When the user presses Shift+Down
    Then AgentView emits Action::HistoryNext
    And the MultiLineInput's buffer is still exactly "draft"

  Scenario: Shift+Left emits Action::SessionPrev without modifying the buffer
    Given an AgentView whose MultiLineInput contains "draft"
    When the user presses Shift+Left
    Then AgentView emits Action::SessionPrev
    And the MultiLineInput's buffer is still exactly "draft"

  Scenario: Shift+Right emits Action::SessionNext without modifying the buffer
    Given an AgentView whose MultiLineInput contains "draft"
    When the user presses Shift+Right
    Then AgentView emits Action::SessionNext
    And the MultiLineInput's buffer is still exactly "draft"

  Scenario: ESC inside AgentView emits Action::BackToBoard
    Given an AgentView whose MultiLineInput contains "draft\nstill drafting"
    When the user presses ESC
    Then AgentView emits Action::BackToBoard
    And the MultiLineInput's buffer is still exactly "draft\nstill drafting"
