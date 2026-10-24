@BUG-213
@phase-1
@tui
@compaction
@context-management
Feature: Compaction footer displays activity without invented work measurements
  """
  Architecture notes:
  The truthful-compaction-activity lifecycle contract applies. Retain the
  CompactionProgress phase/current/total wire shape for existing consumers,
  while the footer displays phase-only activity for any legacy count values.
  Use property-based serialization and presentation coverage and a YAML chrome
  snapshot. Preserve workspace display and supervisor notification priority.
  This specification maps only to truthful_compaction_footer_bug213.rs.
  """

  Scenario: Legacy progress counts never alter the activity presentation
    Given a compaction phase received through the existing progress wire format
    When any legacy current and total values are serialized and delivered to the footer
    Then those fields remain readable by existing snapshot consumers
    And the footer shows the same phase-only activity for every count combination
    And zero totals and counts exceeding totals do not produce a fraction or bar

  Scenario: Truthful activity preserves workspace chrome and supervisor priority
    Given a compacting session with a workspace directory and Git branch
    When its footer is rendered with no pending supervisor message
    Then phase-only activity appears alongside the workspace directory and branch
    When a supervisor message is pending
    Then the supervisor notification takes priority over compaction activity
    And the workspace directory and branch remain visible
