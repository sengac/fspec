@done
@COV-056
@cli
@validation
@coverage
Feature: show-coverage marks @deprecated scenarios in reports

  """
  show-coverage renders a visible DEPRECATED marker for scenarios that are
  deprecated via the @deprecated tag (scenario-level or feature-level) so
  maintainers can tell obsolete requirements apart from live ones. The marker
  is additive: a deprecated scenario keeps its normal coverage status
  (fully-covered/partially-covered/uncovered) and stays counted in stats
  (totalScenarios/coveragePercent unchanged). Features with no @deprecated tag
  render byte-identical output (byte-parity invariant).
  """

  Background: User Story
    As a fspec maintainer
    I want to see which scenarios are deprecated in the coverage report
    So that I can tell obsolete requirements apart from live ones without opening each feature file

  Scenario: Deprecated marker appears in the show-coverage markdown report
    Given a coverage file with one live covered scenario and one scenario tagged @deprecated in its feature file
    When the show-coverage command renders the feature in markdown
    Then the deprecated scenario heading carries a DEPRECATED marker
    And the live scenario heading has no DEPRECATED marker
    And the coverage percentage is computed from all scenarios including the deprecated one

  Scenario: Deprecated marker appears in the show-coverage JSON report
    Given a coverage file with one scenario tagged @deprecated in its feature file
    When the show-coverage command renders the feature in JSON
    Then the deprecated scenario entry carries a deprecated flag
    And its coverageStatus keeps its normal value
    And the stats block counts the deprecated scenario in totalScenarios

  Scenario: Untagged feature files render byte-identical output
    Given a coverage file whose feature file carries no @deprecated tag anywhere
    When the show-coverage command renders the feature in markdown
    Then the output is byte-identical to the pre-deprecation rendering
