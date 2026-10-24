@BUG-210
@phase-1
@tui
@compaction
@context-management
Feature: Automatic compaction progress remains visible on its original session
  """
  Architecture notes:
  The background session already stores compaction activity, but the
  Rust TUI does not poll that state. Publish stored progress updates and clears
  through the existing session-scoped output broadcast and RPC subscriptions.
  Route them into the existing per-session AgentViewStore and footer renderer.
  Keep the original session identity rather than the ephemeral compactor identity
  or whichever session currently has focus. Preserve the existing progress query
  for snapshot consumers, without depending on a polling interval for live updates.
  Terminal cleanup must reach the UI even when cancellation drops the builder
  without a completion event. Continuation policy and completion statistics are
  specified separately and must not be inferred from a progress update or clear.
  BUG-213 supersedes the inherited count presentation: phases describe activity,
  not measured completed work. Keep legacy count fields for wire compatibility,
  but do not render their fractions or a determinate bar.
  """

  Background: User Story
    As a developer using the Rust terminal interface
    I want automatic compaction progress on the session whose context is being compacted
    So that I can tell that interrupted work is still being processed

  Scenario: Automatic compaction shows live progress on the original session
    Given I am viewing a session whose unfinished work triggers automatic compaction
    When its compaction builder starts and reports updated progress
    Then that session visibly shows the current compaction activity phase without counts or a determinate bar
    And successive progress updates appear without waiting for a polling interval
    And the compaction builder does not take focus away from the original session

  Scenario: Background compaction progress stays attached to its original session
    Given automatic compaction is running for one open session
    And I am viewing a different open session
    When the compacting session reports updated progress
    Then the viewed session does not show the other session's compaction progress
    When I focus the compacting session
    Then I see its latest compaction activity phase without counts or a determinate bar
    And its compaction builder has not changed which session I was viewing

  Scenario Outline: Compaction progress disappears when its operation ends
    Given two open sessions each have visible automatic compaction progress
    When automatic compaction for one session <outcome>
    Then that session no longer shows compaction progress
    And the other session keeps its own compaction progress
    And focusing the finished session again does not restore stale progress

    Examples:
      | outcome      |
      | completes    |
      | fails        |
      | is cancelled |
