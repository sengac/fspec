@testing
@providers
@persistence
@provider-abstraction
@PROV-149
Feature: Codex refreshed reminders
  """
  Standalone runtime reminders can refresh without replacing conversation history.
  A compaction DAG is a replacement history boundary, not a refreshable reminder.
  Typed tool and encrypted reasoning state must survive ordinary reminder updates.
  Native provider compaction is unsupported. CMPCT-078 requires a committed
  advancing boundary, not reminder text alone, to replace typed continuation.
  """

  Scenario: Refreshed standalone reminders retain typed continuation
    Given a Codex conversation with a standalone runtime reminder
    And saved tool calls
    When the reminder changes without replacing ordinary user history
    Then typed continuation is retained with the current reminder
    And each real tool output keeps its matching call

  Scenario: Compaction-only history discards old typed continuation
    Given a Codex conversation containing only standalone runtime reminders
    And saved opaque reasoning
    When local compaction commits an advancing boundary before replacing the history with a compaction DAG
    Then the old typed continuation is discarded
    And only the replacement DAG is replayed
