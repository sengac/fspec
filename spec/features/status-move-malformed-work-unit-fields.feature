@done
@BUG-385
@cli
@validation
@workflow
Feature: Status moves reject malformed work-unit fields
  """
  update-work-unit-status (rust/fspec-core/src/commands/update_work_unit_status*)
  read work-unit fields leniently: a missing type became "story", a
  non-array linkedFeatures/blockedBy/children/dependsOn/examples/... read as
  empty, a non-array stateHistory was replaced by an empty array (dropping the
  history) and an unparsable status was ignored. A required field that is
  missing or a present field with the wrong shape now fails the move naming
  the work unit and the field; an absent array field still means none (fspec
  omits empty arrays). Tests:
  rust/fspec-core/tests/bug385_status_move_malformed_fields.rs.
  """

  Background: User Story
    As a developer whose work-units.json was edited by hand or by an old tool
    I want a status move to report a malformed work unit
    So that a gate never passes, or history is never lost, because a field was misread

  Scenario: A work unit without a type fails the move
    Given story "STORY-1" in backlog has no type field
    When the user moves "STORY-1" to specifying
    Then the move fails with an error naming "STORY-1" and "type"
    And "STORY-1" stays in backlog

  Scenario: A malformed blockedBy fails the move
    Given story "STORY-1" in backlog has blockedBy set to the string "BUG-9"
    When the user moves "STORY-1" to specifying
    Then the move fails with an error naming "STORY-1" and "blockedBy"
    And "STORY-1" stays in backlog

  Scenario: A malformed state history fails the move instead of being replaced
    Given story "STORY-1" in backlog has stateHistory set to an object
    When the user moves "STORY-1" to specifying
    Then the move fails with an error naming "STORY-1" and "stateHistory"
    And "STORY-1" stays in backlog
