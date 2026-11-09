@done
@TUI-117
@tui
@agent-view
@tool-execution
@BUG-408
@BUG-459
Feature: Stdin prompt recovery and per-session state
  """
  Builds on the TUI-116 stdin notice and answer box (stdin-notice-and-answer-box.feature).
  Failed sends: Enter in the answer box clears the box and emits Action::ExecStdinSubmit. When
  FspecBackend::write_exec_stdin fails (BUG-362 made the failure a real Err), app/dispatch_exec_stdin.rs emits
  Action::ExecStdinWriteFailed carrying the text and the reason; the reducer puts the text back into that session's
  answer box and sets a red error row ("Not sent: <reason>"). A failed control_exec_stdin emits
  Action::ExecStdinControlFailed and paints "Interrupt failed: <reason>" or "End of input failed: <reason>". Any edit
  in the box clears the error row.
  Auto-close: the backend detector (tools unified_exec/exec_stdin.rs) already clears its request when the command
  exits or prints again, pushing StreamChunk::ExecStdinRequestCleared (a re-probe returning None does the same). On
  that clear the TUI closes the notice and the box, and moves unsent box text into the box history, which Shift+Up
  and Shift+Down walk while the box is focused.
  Mute: Ctrl+X emits Action::ExecStdinMuted; the store records the exec session id for that agent session and
  ignores later requests for it. A different exec session (a new command run) still shows its notice.
  Per session: the view keeps one answer box (input, focus, error row, history) per agent session, and the chat draft
  is already saved per session (RPC-024). Switching sessions shows the incoming session's own notice, box and draft.
  Decisions: spec/attachments/TUI-115/interview-decisions.md (TUI-117 section). Out of scope: a model-driven stdin
  tool.
  """

  Background: User Story
    As a user answering a command's stdin from the agent view
    I want to keep my stdin answer when a write fails, have the notice close itself when the command moves on, mute a nagging run, and keep each session's stdin state to itself
    So that I never lose typed input or leak it into another session

  Scenario: A failed write keeps the text in the answer box and shows a red error
    Given command "git commit" in exec session "exec-abc" may be waiting for input
    And writing to the command fails with "broken pipe"
    And the user has typed "y" into the answer box
    When the user presses Enter
    Then the answer box still shows "y"
    And a red row in the answer box reads "Not sent: broken pipe"

  Scenario: A failed interrupt shows a red error in the answer box
    Given command "git commit" in exec session "exec-abc" may be waiting for input
    And interrupting the command fails with "no such process"
    And the answer box is open
    When the user presses Ctrl+C
    Then a red row in the answer box reads "Interrupt failed: no such process"

  Scenario: Editing the answer box clears the error
    Given a failed write left "y" and the error "Not sent: broken pipe" in the answer box
    When the user types "e"
    Then the answer box shows "ye"
    And no error row is shown

  Scenario: The command printing again closes the notice and keeps unsent text in the box history
    Given command "git commit" in exec session "exec-abc" may be waiting for input
    And the user has typed "partial" into the answer box
    When the command prints again and its stdin request is cleared
    Then the notice and the answer box close
    And when the command asks again the answer box opens empty
    And Shift+Up in the answer box recalls "partial"

  Scenario: Ctrl+X mutes further notices for the same command run
    Given command "git commit" in exec session "exec-abc" may be waiting for input
    When the user presses Ctrl+X
    And exec session "exec-abc" asks for input again
    Then no notice is shown
    And when exec session "exec-def" asks for input the notice is shown

  Scenario: Each session keeps its own notice, answer box and chat draft
    Given sessions "s-1" and "s-2" are open
    And command "git commit" in session "s-1" may be waiting for input
    And command "cargo build" in session "s-2" may be waiting for input
    And in session "s-1" the chat draft is "one" and the answer box holds "y"
    When the user switches to session "s-2"
    Then the "cargo build" notice is shown with the answer box closed and an empty chat draft
    And opening the answer box in session "s-2" shows it empty
    When the user switches back to session "s-1"
    Then the answer box is focused and shows "y"
    And the chat draft is "one"

  Scenario: Without a runtime an answer is put back in the answer box
    Given command "git commit" in exec session "exec-abc" may be waiting for input and there is no tokio runtime
    When the user submits "y" to the command
    Then the answer box still shows "y"
    And a red row in the answer box reads "Not sent: no tokio runtime: write_exec_stdin not sent"
