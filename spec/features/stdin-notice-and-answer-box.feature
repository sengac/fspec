@BUG-582
@done
@TUI-116
@tui
@agent-view
@tool-execution
Feature: Stdin notice and answer box
  """
  A quiet-command stdin request (fspec's 3-second silence guess, ExecStdinRequest in the per-session store slot) no
  longer replaces the composer. The focused agent pane paints a one-line notice above the composer (or above the busy
  stack while the turn runs): "<command> may be waiting for input · Ctrl+O answer · Ctrl+X dismiss". The notice takes
  no focus: every key except Ctrl+O and Ctrl+X keeps its chat meaning, so typing edits the chat draft and Esc still
  interrupts a running turn.
  Ctrl+O opens a separate stdin answer box with its own MultiLineInput buffer (AgentView field, views/agent/
  stdin_answer_box.rs), so the chat draft can never be written to the command. The box paints a header, its input
  and the hint "Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input". Box keys (views/agent/
  exec_stdin_keys.rs): Enter sends the line through FspecBackend::write_exec_stdin and clears the box; Esc returns
  focus to the chat draft and keeps the notice and the box text; Ctrl+C sends
  FspecBackend::control_exec_stdin(Interrupt) (SIGINT to the command only, never Action::Interrupt); Ctrl+D sends
  control_exec_stdin(Eof). Action::ExecStdinControl is reduced in app/dispatch_exec_stdin.rs.
  Ctrl+X with the chat focused clears the slot only (Action::ExecStdinDismissed); nothing reaches the command.
  Precedence: a HITL prompt wins over the notice and the box, a pause prompt is painted without the notice, and ghost
  panes never paint either. A re-probe that returns no request clears the slot, which closes the notice and the box.
  Supersedes the TUI-layer scenarios of exec-stdin-prompt.feature (TOOL-022 P2). Decisions:
  spec/attachments/TUI-116/interview-decisions.md. Follow-up: TUI-117 (failed writes, auto-close, per-session state).
  """

  Background: User Story
    As a user chatting with an agent while one of its commands runs
    I want to see when the command may be waiting for input and answer it in its own box
    So that I can answer the command without losing or leaking my chat draft

  Scenario: The notice sits above the composer and leaves the chat draft in focus
    Given the focused session's turn is running
    And command "git commit" may be waiting for input
    When the user types "hello"
    Then the row above the activity row reads "git commit may be waiting for input · Ctrl+O answer · Ctrl+X dismiss"
    And the chat draft is "hello"

  Scenario: Ctrl+O opens a separate answer box with its own buffer
    Given the focused session's turn is running
    And command "git commit" may be waiting for input
    And the chat draft is "hello"
    When the user presses Ctrl+O and types "y"
    Then the answer box shows "y" with the hint "Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input"
    And the chat draft is still "hello"

  Scenario: Enter in the answer box writes the line to the command and sends no chat message
    Given the focused session's turn is running
    And command "git commit" in exec session "exec-abc" may be waiting for input
    And the user has typed "y" into the answer box
    When the user presses Enter
    Then the backend writes "y" to exec session "exec-abc"
    And no chat message is sent
    And the notice and the answer box close

  Scenario: Esc in the answer box returns to the chat draft and keeps both drafts
    Given the focused session's turn is running
    And command "git commit" may be waiting for input
    And the chat draft is "draft"
    And the user has typed "n" into the answer box
    When the user presses Esc
    Then the chat draft is "draft" and the notice is still shown
    And the session turn is not interrupted
    And pressing Ctrl+O again shows "n" in the answer box

  Scenario: Ctrl+C in the answer box interrupts only the command
    Given the focused session's turn is running
    And command "git commit" in exec session "exec-abc" may be waiting for input
    And the answer box is open
    When the user presses Ctrl+C
    Then the backend sends an interrupt to exec session "exec-abc"
    And the session turn is not interrupted

  Scenario: Ctrl+D in the answer box ends the command's input
    Given the focused session's turn is running
    And command "git commit" in exec session "exec-abc" may be waiting for input
    And the answer box is open
    When the user presses Ctrl+D
    Then the backend sends end of input to exec session "exec-abc"
    And the TUI keeps running

  Scenario: Esc with the chat focused still interrupts the agent turn
    Given the focused session's turn is running
    And command "git commit" may be waiting for input
    When the user presses Esc
    Then the session turn is interrupted
    And nothing is sent to the command

  Scenario: Ctrl+X dismisses the notice without touching the command
    Given the focused session's turn is running
    And command "git commit" may be waiting for input
    When the user presses Ctrl+X
    Then the notice is no longer shown
    And nothing is sent to the command

  Scenario: A HITL prompt wins over the notice
    Given the focused session has a pending HITL request
    When command "git commit" may be waiting for input
    Then the input area shows the HITL prompt
    And the notice is not shown
    And the notice is shown once the HITL prompt is cleared and the request arrives again

  Scenario: Ghost panes show no notice
    Given agent session A has command "cmd-a" waiting for input
    And agent session B is focused while agent session A is a ghost pane
    When the agent view paints all panes
    Then neither pane shows a notice for "cmd-a"
    And pane A shows the notice for "cmd-a" once it is focused

  Scenario: A re-probe with no request closes the notice and the answer box
    Given command "git commit" in exec session "exec-gone" may be waiting for input
    And the answer box is open
    When the exec session has exited and the agent view re-probes the backend
    Then the notice and the answer box are no longer shown
    And the chat composer is shown again
