@done
@BUG-434
@cli
@validation
@workflow
Feature: Linked feature files match the whole work-unit tag
  """
  update-work-unit-status's prefill, temporal and review gates find a work
  unit's feature files with linked_feature_files
  (rust/fspec-core/src/commands/update_work_unit_status/prefill.rs). It used to
  take every feature whose text contained "@<id>", so BUG-1's gates read
  features tagged @BUG-10..@BUG-19. A feature is now linked only when a tag
  line (a line starting with @) has the tag as a whole whitespace-separated
  token. Tests: rust/fspec-core/tests/bug434_linked_feature_tag_matching.rs.
  """

  Background: User Story
    As a developer moving work units through fspec's gates
    I want each gate to read only the features tagged with my work unit
    So that another card's feature never blocks or passes my card

  Scenario: A feature tagged with a longer id that starts with the work unit id is not linked
    Given bug "BUG-1" is in backlog
    And "spec/features/other.feature" is tagged "@BUG-10" and contains a user-story role placeholder
    When the user moves "BUG-1" to specifying
    Then the move succeeds

  Scenario: A feature carrying the work unit tag among other tags is linked
    Given bug "BUG-1" is in backlog
    And "spec/features/mine.feature" has the tag line "@done @BUG-1 @cli" and contains a user-story role placeholder
    When the user moves "BUG-1" to specifying
    Then the move fails with the prefill placeholder error
