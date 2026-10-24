@done
@session
@rpc
@tui
@persistence
@agent-view
@BUG-244
Feature: Transcript hydration live state
  """
  Historical content hydration preserves current provider waiting chrome.
  Fresh content notifications update waiting independently from rendering,
  while periodic recovery retains bounded completed-task state.
  """

  Scenario: Historical hydration preserves current provider waiting state
    Given an open session whose original transcript page has not reached the TUI
    When a current provider waiting notification arrives before the historical page is applied
    Then the original text renders without clearing the current provider waiting indicator

  Scenario: Idle transcript polling retains bounded task state
    Given a hydrated idle session with no additional transcript events
    When many periodic transcript polls complete in the production action path
    Then completed page tasks are released instead of accumulating with elapsed idle time

  Scenario: Live content notifications dismiss provider waiting without rendering twice
    Given a hydrated tab receiving live provider waiting notifications
    When fresh response content arrives through either subscribed transport
    Then the waiting indicator clears and the response renders exactly once
