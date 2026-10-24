@done
@session
@rpc
@tui
@persistence
@agent-view
@BUG-244
Feature: Lossless transcript TUI hydration
  """
  Each tab owns one authoritative transcript cursor and a distinct hydration
  identity. Content notifications wake cursor replay; they never append another
  copy. Live notifications update transient waiting chrome independently from
  historical rendering. Failed pages preserve committed display state and expose
  a scoped retry.
  """

  Scenario: Reopened tabs render the complete transcript without duplicate tool cards
    Given a real saved long transcript with a complete tool result
    When the TUI resumes it twice and reopens its tab using either transport
    Then its rendered original text and full tool result remain complete and occur once
    And original timestamps remain associated with the rendered turns

  Scenario: Late hydration cannot corrupt a reopened tab or another session
    Given a real session whose hydration response has not reached the TUI
    When its tab is closed and reopened before that response is applied
    Then only the current tab transcript renders once and no other session receives its chunks

  Scenario: Lost live wakeups recover without duplicating the displayed transcript
    Given an open hydrated session with original text
    When live output arrives without a notification and the transcript is polled
    And the same live notification is received twice
    Then the original and new text each render exactly once

  Scenario: Failed hydration preserves displayed content and exposes retry state
    Given an open hydrated session with original text
    When its next transcript page fails
    Then the original text remains visible with a session scoped incomplete transcript notice

  Scenario: Resuming a previously restored active session preserves runtime state
    Given a cold restored session with new live model messages and token counters
    When the active session is manually resumed again
    Then its live model messages counters and transcript remain unchanged
