@done
@BUG-645
@cli
@feature-management
@coverage-tracking
@rust
Feature: Bulk deletion leaves a clean spec tree
  """
  delete-features removed the feature files but left their .feature.coverage
  sidecars behind, so fspec audit then reported orphan coverage files (the
  audit helper cleaned them up with audit --fix during CLEAN-010). Now each
  deleted feature's coverage file is deleted with it, under the sidecar
  transaction (BUG-477); the result lists the deleted coverage files in
  coverageFiles, a dry run lists the ones it would delete, and the CLI prints
  them.
  delete-scenarios computed its own block ranges (a port of the TypeScript
  command): a block ended at the next Scenario or Background, otherwise at the
  end of the file. Deleting the last scenarios left the blank line before them,
  so a formatter-clean feature failed the format check, and a Rule after the
  deleted scenario was swept away with everything to the end of the file. Now
  delete-scenarios removes each block exactly as delete-scenario does
  (BUG-486, one shared block function in
  rust/fspec-core/src/commands/delete_scenario.rs): the block ends just before
  the next scenario-level header (Scenario, Scenario Outline, Example,
  Background, Rule) and its tag and comment lines, and deleting the last
  scenarios leaves the previous content followed by one newline.
  Code: rust/fspec-core/src/commands/delete_features.rs,
  rust/fspec-core/src/commands/delete_scenarios.rs,
  rust/fspec-core/src/commands/delete_scenario.rs and the CLI bridge
  rust/fspec/src/delete_features.rs. Tests:
  rust/fspec-core/tests/bug645_bulk_deletion_leaves_a_clean_spec_tree.rs and
  rust/fspec/tests/bug645_delete_features_cli_lists_coverage_files.rs.
  """

  Background: User Story
    As a maintainer removing features and scenarios with fspec
    I want to have delete-features and delete-scenarios leave no orphan coverage file, no trailing blank line and no swept-away Rule
    So that a bulk deletion leaves the spec tree passing fspec audit without hand repairs

  Scenario: Deleting features deletes their coverage files
    Given feature "a" with a coverage file and feature "b" without one are tagged @gone, and feature "c" with a coverage file is not
    When I delete the features tagged @gone
    Then a.feature, its coverage file and b.feature are gone
    And c.feature and its coverage file are untouched
    And the result lists spec/features/a.feature.coverage as the only deleted coverage file

  Scenario: A dry run lists the coverage files it would delete
    Given feature "a" with a coverage file is tagged @gone
    When I delete the features tagged @gone as a dry run
    Then the result lists spec/features/a.feature.coverage as a coverage file it would delete
    And a.feature and its coverage file are still on disk

  Scenario: The CLI prints the deleted coverage files
    Given feature "a" with a coverage file is tagged @gone
    When I run fspec delete-features --tag @gone
    Then stdout lists spec/features/a.feature.coverage under "Deleted coverage files:"
    And the coverage file is gone

  Scenario: The CLI dry run prints the coverage files it would delete
    Given feature "a" with a coverage file is tagged @gone
    When I run fspec delete-features --tag @gone --dry-run
    Then stdout lists spec/features/a.feature.coverage under "Would delete 1 coverage file(s):"
    And the coverage file is still on disk

  Scenario: Deleting the last scenario leaves a formatter-clean file
    Given a formatter-clean feature whose last scenario "Two" is tagged @drop
    When I delete the scenarios tagged @drop
    Then the file ends with the last step of scenario "One" followed by one newline
    And the file equals the formatter's output

  Scenario: Deleting the last two scenarios leaves a formatter-clean file
    Given a formatter-clean feature whose scenarios "Two" and "Three" after scenario "One" are tagged @drop
    When I delete the scenarios tagged @drop
    Then the file ends with the last step of scenario "One" followed by one newline
    And the file equals the formatter's output

  Scenario: Deleting every scenario leaves a formatter-clean file
    Given a formatter-clean feature with a Background whose only scenarios are tagged @drop
    When I delete the scenarios tagged @drop
    Then the file ends with the last step of the Background followed by one newline
    And the file equals the formatter's output

  Scenario: A Rule after a deleted scenario is kept
    Given a feature with scenario "One", scenario "Two" tagged @drop, then Rule "R" holding scenario "Three"
    When I delete the scenarios tagged @drop
    Then scenario "Two" is gone
    And scenario "One", Rule "R" and scenario "Three" are unchanged
