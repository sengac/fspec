@BUG-213
@phase-1
@tui
@compaction
@context-management
Feature: Compaction activity does not imply measured work completion
  """
  Architecture notes:
  DAG construction has no measured completed-work contract. Existing lifecycle
  boundaries publish activity on the original session, using the existing
  CompactionProgress phase/current/total transport and snapshot query. Unknown
  work counts use current=0 and total=0, not active message counts, approximate
  turns, or a synthetic one-operation denominator. Retain these wire fields for
  existing consumers; the TUI treats all count values as legacy metadata and
  renders phase-only activity, including when receiving an older producer.
  Reuse the existing busy animation; do not introduce an additional spinner,
  percentage, ETA, heartbeat, or fabricated intermediate completed-work count.
  Report only phases reached by the actual operation. Preserve embedded and
  WebSocket session routing, snapshot reads, and terminal clears. Activity or its
  removal is not evidence of a committed generation. Authoritative final token
  basis and available coverage remain owned by committed-context reporting.
  Tests must observe an actual pending builder's initial event before supplying
  any test-driven phase update. Escape cancellation semantics are specified
  separately; this feature preserves terminal activity cleanup.
  """

  Background: User Story
    As a developer waiting for conversation compaction
    I want truthful activity on the session being compacted
    So that placeholder counts do not misrepresent completed work

  Scenario: A pending automatic builder reports activity rather than message-count progress
    Given an original session with 153 active messages and a real pending compaction builder
    When automatic compaction starts that builder
    Then the original session shows the builder's current activity phase
    And the producer reports unknown work counts rather than 153 completed-work units
    And the footer shows no completed-work fraction, percentage, ETA, or determinate bar
    And no test-supplied progress update is needed to make activity visible
    And the original session retains focus

  Scenario Outline: Live activity remains attached to its original session
    Given two open sessions connected through the <transport> backend
    And the background session has a real pending automatic compaction builder
    When the original session reports a reached activity phase
    Then the viewed session does not show the background session's activity
    And the builder does not change session focus
    When I focus the compacting session
    Then I see its latest phase-only activity without waiting for a polling interval

    Examples:
      | transport |
      | embedded  |
      | WebSocket |

  Scenario Outline: Terminal cleanup removes activity without inventing success
    Given two sessions with compaction activity through both supported transports
    When one session's compaction <outcome>
    Then only that session's activity is cleared in the backend and footer
    And refocusing that session does not restore stale activity
    And an activity clear alone adds no successful-compaction notice or reduction badge
    And a committed completion retains its authoritative token basis and available final coverage

    Examples:
      | outcome      |
      | completes    |
      | fails        |
      | is cancelled |
