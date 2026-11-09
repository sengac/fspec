@done
@BUG-522
@cli
@validation
@rust
Feature: fspec validate reports orphan coverage files
  """
  Nine spec/features/*.feature.coverage files had no .feature (found
  2026-11-05 as seven, nine at HEAD): six features were deleted by earlier TUI
  and napi refactors (99fdc3bb3, 50d4d5932, ac93e854e, 273d6afb3) without
  their coverage; add-kotlin-and-dart-language-support-to-ast-research-tool
  and coverage-validation-missing-for-implementing-to-validating-transition
  were superseded by renamed features; tui-clear-session-context.feature was
  never committed. Nothing reported them. The orphans are deleted, and
  'fspec validate' (all features, rust/fspec-core/src/commands/validate.rs)
  now reports every coverage file without its feature as invalid.
  Tests: rust/fspec-core/tests/bug522_validate_reports_orphan_coverage.rs.
  """

  Background: User Story
    As a maintainer keeping spec/features consistent
    I want to have fspec validate report coverage files whose feature no longer exists, and remove the orphans left in the repository
    So that coverage data never outlives the feature it describes unnoticed

  Scenario: A coverage file without its feature fails validation
    Given a project with a valid feature and a coverage file "gone.feature.coverage" with no "gone.feature"
    When I validate all feature files
    Then validation fails naming "spec/features/gone.feature.coverage" as having no feature file

  Scenario: Coverage files that match their features pass validation
    Given a project with a valid feature and its coverage file
    When I validate all feature files
    Then validation passes

  Scenario: The repository has no orphan coverage files
    Given the repository's spec/features directory
    When I list the coverage files
    Then every coverage file has its feature file
