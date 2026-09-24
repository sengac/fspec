@done
@COV-056
@cli
@validation
@coverage
Feature: @deprecated tag exempts scenarios from the coverage gates

  """
  The @deprecated tag (scenario-level or feature-level) marks a requirement that
  is no longer required (e.g. obsolete after a migration). Such scenarios must
  stop requiring coverage: the update-work-unit-status done-gate and the
  implementing->validating gate exclude them from the uncovered / without-impl
  lists. The tag is read from the .feature file (Gherkin source of truth) via
  the lenient parser; no coverage-file schema change. Feature-level @deprecated
  (Feature: header tag run) deprecates every scenario in the feature. Feature
  file parse failure keeps current behavior (deprecated check silently
  skipped, parity with the existing stale-scenario check).
  """

  Background: User Story
    As a fspec maintainer
    I want to mark obsolete scenarios and features with @deprecated
    So that deprecated requirements stop blocking work units and stop inflating coverage gaps

  Scenario: Done-gate exempts a scenario-level @deprecated scenario without test mapping
    Given a work unit with status "validating" linked to a feature file
    And the feature has one live scenario with test mapping and one scenario tagged @deprecated with no test mapping
    When the dispatcher moves the work unit to "done"
    Then the transition succeeds
    And the work unit status becomes "done"
    And no error mentions the deprecated scenario as uncovered

  Scenario: Done-gate still blocks on an uncovered live scenario
    Given a work unit with status "validating" linked to a feature file
    And the feature has one untagged scenario with no test mapping
    When the dispatcher moves the work unit to "done"
    Then the transition fails
    And the error lists the untagged scenario as uncovered

  Scenario: Validating gate exempts a @deprecated scenario without implementation mapping
    Given a work unit with status "implementing" linked to a feature file
    And the feature has one scenario tagged @deprecated whose test mapping has no impl mapping
    When the dispatcher moves the work unit to "validating"
    Then the transition succeeds
    And the work unit status becomes "validating"

  Scenario: Feature-level @deprecated deprecates every scenario in the feature
    Given a work unit with status "validating" linked to a feature file tagged @deprecated on the Feature header
    And the feature has three scenarios, none with test mappings
    When the dispatcher moves the work unit to "done"
    Then the transition succeeds
    And no scenario in the feature is reported as uncovered
