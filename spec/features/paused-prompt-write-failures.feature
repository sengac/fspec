@done
@BUG-395
@tui
@pause-integration
Feature: Paused prompt write failures
  """
  rust/fspec-tui/src/app/dispatch_pause_hitl.rs sends the user's pause answer (pause_confirm), tool-approval choice (pause_triple), pause resume (pause_resume) and question answer (send_hitl_response) to the backend in a spawned task after clearing the prompt slot. A failed write is sent to the app as Action::PausedWriteFailed { session_id, write, error }; dispatch_pause_route.rs logs it at error level, pushes a red "[error] {write} failed: {error}" notice into the session scrollback, and re-runs the Paused probe (handle_pause_chunk) so a session that is still paused shows its prompt again. Without a tokio runtime the writer reports the same notice synchronously and leaves the prompt slot untouched. Test file: rust/fspec-tui/tests/paused_prompt_write_failures_bug395.rs.
  TUI-132 (2026-11-09): Action::PauseResumed and handle_pause_resumed are removed (nothing emitted them), with the pause resume scenario.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A failed pause answer, tool-approval choice, pause resume or question answer is logged at error level and shown as a red notice in the session scrollback naming the failed write
  #   2. After a failed write the prompt is read back from the backend, so a session that is still paused shows its prompt again and the user can answer again
  #   3. Without a tokio runtime the write is not attempted, the prompt is not cleared, and the failure is reported the same way
  #
  # EXAMPLES:
  #   1. The user accepts a confirm pause prompt and pause_confirm fails with "connection reset": the scrollback shows "pause_confirm failed" and the confirm prompt is back
  #   2. The user submits a question answer with no tokio runtime: the question stays and the scrollback shows "send_hitl_response failed"
  #
  # ========================================
  Background: User Story
    As a user answering a paused agent session in the TUI
    I want to see when my answer did not reach the agent and get the prompt back
    So that the session is never left stuck waiting for an answer that was lost

  Scenario: A failed pause answer is reported and the pause prompt comes back
    Given a session paused with a confirm pause prompt shown
    And sending a pause answer fails with "connection reset"
    When the user accepts the pause prompt
    Then the session scrollback shows that pause_confirm failed with "connection reset"
    And the confirm pause prompt is shown again

  Scenario: A failed tool-approval choice is reported and the approval prompt comes back
    Given a session paused with a tool-approval prompt shown
    And sending a tool-approval choice fails with "connection reset"
    When the user approves the tool call once
    Then the session scrollback shows that pause_triple failed with "connection reset"
    And the tool-approval prompt is shown again

  Scenario: A failed question answer is reported and the question comes back
    Given a session paused with a question shown
    And sending a question answer fails with "connection reset"
    When the user submits an answer
    Then the session scrollback shows that send_hitl_response failed with "connection reset"
    And the question is shown again

  Scenario: Without a runtime a pause answer is reported and the pause prompt stays
    Given a session with a confirm pause prompt shown and no tokio runtime
    When the user accepts the pause prompt
    Then the session scrollback shows that pause_confirm failed with "no tokio runtime"
    And the confirm pause prompt is still shown

  Scenario: Without a runtime a question answer is reported and the question stays
    Given a session with a question shown and no tokio runtime
    When the user submits an answer
    Then the session scrollback shows that send_hitl_response failed with "no tokio runtime"
    And the question is still shown
