@done
@BUG-394
@tui
@pause-integration
Feature: Paused probe read failures
  """
  The Paused probe in rust/fspec-tui/src/app/dispatch_pause_hitl.rs reads get_pause_state, get_hitl_request and get_exec_stdin_request in parallel. A failed pause or question read is sent to the app as Action::PausedReadFailed; the reducer logs it at error level and pushes a red notice into that session's scrollback. A failed question read withholds the pause prompt because the question takes precedence. Every send in the probe goes through dispatch_exec_stdin_recovery::send_to_app, which logs a closed app loop.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A failed get_pause_state or get_hitl_request read during the Paused probe is reported at error level and as a red notice in that session's scrollback, never logged at debug and folded into None.
  #   2. When the question read fails, a fetched pause prompt is not shown in its place, because the question would have taken precedence; every Paused probe send goes through the logging send helper.
  #
  # EXAMPLES:
  #   1. get_hitl_request fails with 'decode failed' and there is no pause: no prompt is set and the scrollback shows the read failure.
  #   2. get_pause_state fails and there is no question: no pause prompt is set and the failure is shown.
  #   3. get_hitl_request fails while get_pause_state returns a confirm prompt: no pause prompt is shown and the failure is shown.
  #
  # ========================================
  Background: User Story
    As a TUI user whose agent session paused
    I want to see when the paused session's prompt could not be read
    So that a stuck session never looks idle without a reason

  Scenario: A failed question read is reported and sets no prompt
    Given a session paused with no pause prompt
    When the question read fails during the Paused probe
    Then no question or pause prompt is set for the session
    And the session scrollback shows the failed question read

  Scenario: A failed pause read is reported and sets no prompt
    Given a session paused with no question
    When the pause read fails during the Paused probe
    Then no question or pause prompt is set for the session
    And the session scrollback shows the failed pause read

  Scenario: A failed question read does not let a pause prompt take its place
    Given a session paused with a confirm pause prompt
    When the question read fails during the Paused probe
    Then no pause prompt is shown for the session
    And the session scrollback shows the failed question read
