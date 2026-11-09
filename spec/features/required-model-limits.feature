@done
@PROV-165
@providers
@context-window
Feature: Work that needs model limits refuses unavailable limits
  """
  Model limits may be unavailable (unavailable-model-limits.feature). A compaction threshold
  exists only when both limits are: resolve_compaction_threshold takes Option inputs and returns
  Option<u64>. Work that needs numbers asks codelet_cli::model_limits::required_limits, which fails
  with ModelLimitUnavailable naming the missing limit and the selected model. Prompt preflight
  (before any request), the compactor's publication and turn compaction use it, so nothing is
  admitted, budgeted or compacted against a limit nobody declared.
  """

  Background: User Story
    As a session owner whose model has no published limits
    I want work that needs a context window or output limit to fail naming the missing limit
    So that I can set the limit instead of getting requests budgeted against 0

  Scenario: No compaction threshold exists without both limits
    Given a context window of 200000 and an output limit of 8192
    When the compaction threshold is resolved with either limit unavailable
    Then no compaction threshold exists
    And with both limits available the threshold is positive

  Scenario: Work that needs model limits fails naming the unavailable limit
    Given a Codex provider manager for "codex/unknown-model" with no catalog or user limits
    When the limits a prompt needs are requested
    Then the request fails with an error naming the unavailable context window
    And the error names the model "codex/unknown-model"
