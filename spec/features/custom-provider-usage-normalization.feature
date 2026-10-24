@done
@CMPCT-065
@providers
@context-management
@compaction
@phase-1
Feature: Custom providers declare their additive usage contract
  """
  Custom providers declare disjoint input, cache, output and reasoning categories.
  Normalize once at that boundary without reducing occupancy for cost discounts.
  Missing measurements remain unknown; preserve immutable originals and the
  committed DAG without alternate requests or substitute memory.
  """

  Scenario: Custom providers declare their additive usage contract
    Given a custom provider reports disjoint input cache output and reasoning categories
    When custom usage reaches context accounting
    Then each category contributes once and missing measurements remain unknown
