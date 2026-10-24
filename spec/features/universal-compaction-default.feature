@context-management
@compaction
@cli
@phase-1
Feature: Every model defaults to compaction at eighty percent
  """
  The same default applies to Claude, Gemini, OpenAI, custom models and
  missing model identities. Explicit token or percentage overrides retain
  priority. Every result remains bounded by usable context after output
  reservation, and missing capacity or output limits cannot authorize use.
  """

  Scenario: All model identities receive the same default
    Given known, custom and absent model identities with explicit model limits
    And no configured compaction threshold
    When their compaction thresholds are resolved
    Then each default is eighty percent of context bounded by usable context

  Scenario: Explicit overrides retain priority over the universal default
    Given known, custom and absent model identities with explicit model limits
    And configured token or percentage thresholds
    When their compaction thresholds are resolved
    Then configured thresholds take priority and remain bounded by usable context

  Scenario: Missing limits remain unavailable under the universal default
    Given known, custom and absent model identities with missing capacity or output limits
    When their compaction thresholds are resolved
    Then no usable compaction threshold is available
