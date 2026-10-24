@done
@session
@rpc
@tui
@persistence
@agent-view
@BUG-244
Feature: Lossless transcript delivery
  """
  Existing session output is the runtime journal, not a best-effort broadcast.
  Pages identify a session generation, next sequence and fixed snapshot boundary.
  Both transports expose the same contract. Live notifications only wake replay;
  periodic catch-up recovers lost notifications without replaying user actions.
  Restoration installs history without publishing it as fresh agent activity.
  """

  Scenario: Long transcript resumes exactly once across both transports
    Given a real session containing more than 2479 ordered chunks including tool calls and results
    When its transcript is hydrated through embedded and WebSocket transports in bounded pages
    And more output arrives between pages and hydration is repeated
    Then every original and new event is available once in source order with complete tool content

  Scenario: Broadcast lag recovers from the authoritative cursor
    Given a real session with a subscriber that falls more than 256 events behind
    When the subscriber recovers using its transcript cursor
    Then the complete missed content and final tail are delivered exactly once

  Scenario: Invalid cursors fail visibly without corrupting an existing transcript
    Given an existing transcript and its generation cursor
    When a page uses an invalid generation or sequence
    Then a typed recoverable failure preserves the already displayed content

  Scenario: Repeated restoration does not rebroadcast history
    Given a saved real session and a live output subscriber
    When the same saved history is restored twice
    Then the transcript contains one copy and no historical content is broadcast as new activity

  Scenario: Large Unicode tool results cross bounded pages without truncation
    Given a real session with a Unicode tool result larger than 256 KiB
    When the transcript crosses bounded pages through both transports
    Then no page exceeds 64 KiB and the complete original tool result is available once
