@CMPCT-065
@providers
@context-management
@compaction
@phase-1
Feature: Responses continuation usage preserves physical evidence
  """
  Architecture: aggregate only individually consistent physical usage with
  checked arithmetic. Unknown cache or reasoning detail stays unknown rather
  than becoming measured zero. Rejected physical evidence cannot be repaired
  by a later response or wrapping integer arithmetic.
  """

  Scenario: Invalid physical usage cannot become valid in aggregate
    Given two physical Responses measurements whose inconsistent totals cancel
    When their usage is aggregated
    Then aggregation rejects the invalid physical evidence

  Scenario: Individually valid usage cannot overflow its aggregate
    Given individually valid Responses measurements whose combined totals exceed the integer limit
    When their usage is aggregated
    Then aggregation returns an error without panic or wrapped counters

  Scenario: Missing detail remains unknown in either operand order
    Given valid Responses measurements with absent cache and reasoning detail
    When they are aggregated with explicitly measured detail in either order
    Then both aggregate details remain unknown

  Scenario: Explicit detail and inclusive totals aggregate exactly
    Given valid Responses measurements with explicit cache and reasoning detail
    When their usage is aggregated
    Then inclusive totals and measured subsets sum exactly without double counting
