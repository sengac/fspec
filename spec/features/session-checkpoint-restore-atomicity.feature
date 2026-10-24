@done
@CMPCT-061
@CMPCT-062
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Failed provider checkpoint restore preserves the complete session state
  """
  A real SessionManager restore must validate retained provider checkpoint state
  before installing the committed generation or saved counters. Failure keeps
  inference blocked without replacing messages, checkpoint, replay installation,
  token tracking or background output. A subsequent valid retry remains usable.
  """

  Scenario Outline: Invalid retained checkpoints cannot partially restore a session
    Given a fresh-process session with a committed generation and distinct existing runtime state
    When persisted retained checkpoint metadata has <failure>
    Then restore fails and session inference remains blocked
    And messages checkpoint counters replay installation and output remain unchanged
    And no physical request or completion is produced
    When the authoritative valid replay is restored
    Then the saved generation counters and history become usable

    Examples:
      | failure              |
      | unsupported version  |
      | unsupported protocol |
      | null checkpoint      |
      | future generation    |
