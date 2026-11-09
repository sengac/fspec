@done
@BUG-384
@cli
@validation
@workflow
Feature: Prefill gate checks every linked feature
  """
  check_prefill (rust/fspec-core/src/commands/update_work_unit_status/prefill.rs)
  ran detect_prefill on features.first() only, and linked_feature_files
  returns features in read_dir order (unspecified), so with two linked
  features a placeholder in the other one never blocked the move. The gate
  now checks every linked feature in path order and names the file whose
  placeholders block the move. Tests:
  rust/fspec-core/tests/bug384_prefill_gate_all_linked_features.rs.
  """

  Background: User Story
    As a developer whose work unit spans several feature files
    I want the prefill gate to check all of them
    So that placeholder text in any linked feature blocks the move

  Scenario: A placeholder in the first linked feature blocks the move
    Given bug "BUG-1" is in backlog
    And "spec/features/a.feature" and "spec/features/z.feature" are both tagged "@BUG-1"
    And only "spec/features/a.feature" contains a user-story placeholder
    When the user moves "BUG-1" to specifying
    Then the move fails with an error naming "a.feature"

  Scenario: A placeholder in a later linked feature blocks the move
    Given bug "BUG-1" is in backlog
    And "spec/features/a.feature" and "spec/features/z.feature" are both tagged "@BUG-1"
    And only "spec/features/z.feature" contains a user-story placeholder
    When the user moves "BUG-1" to specifying
    Then the move fails with an error naming "z.feature"
