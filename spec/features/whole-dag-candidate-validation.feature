@done
@CMPCT-070
@compaction
@agent-core
@phase-1
@validation
Feature: Whole DAG candidate validation
  """
  One core validator consumes immutable persisted-turn coordinates and required
  coverage supplied by the source owner. It returns the unchanged candidate and
  complete metadata, or a typed rejection. It does not salvage or install nodes.
  Cross-depth overlap is intentional. Route owners must validate before promotion.
  """

  Background: User Story
    As an agent retaining hierarchical memory
    I want the entire DAG validated against authoritative persisted history
    So that malformed memory never replaces a committed generation

  Scenario: Validate persisted coordinates after active history shrinks
    Given persisted source turns extend beyond the active history
    When a complete candidate covers the required persisted prefix
    Then its original text and exact persisted bounds are accepted unchanged

  Scenario: Reject malformed candidates without salvaging valid nodes
    Given a valid node already covering the complete required prefix and additional malformed structure
    When the whole candidate is validated
    Then validation fails without returning a surviving subset

  Scenario: Reject invalid provenance and missing coverage
    Given candidates with inverted bounds unknown turns or coverage gaps
    When the whole candidate is validated against persisted source identities
    Then each candidate is rejected without clamping or invented coverage

  Scenario: Preserve cross-depth overlap but reject same-depth overlap
    Given candidates with overlapping hierarchical turn ranges
    When the whole candidate is validated
    Then cross-depth overlap is accepted and same-depth overlap is rejected

  Scenario: Reject invalid existing marked DAG as invalid source
    Given an existing marked DAG containing malformed nodes
    When the source preservation contract is captured
    Then a typed invalid-source failure is returned rather than zero coverage
