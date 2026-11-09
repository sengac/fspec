@BUG-582
@done
@PROV-165
@providers
@context-window
Feature: Unavailable model limits are reported explicitly
  """
  A model's context window and output limit come from a user value, the model catalog or a limit
  the provider declares. When none of them exists the limit is unavailable, and that is a typed
  state, not the number 0: ModelLimitsResolver::default_context_window/default_max_output_tokens
  return Option<usize>, resolve_model_limits/resolve_context_window/resolve_max_output_tokens
  return Option<usize>, and ProviderManager::context_window/max_output_tokens return Option<usize>.
  Work that needs a number and the compaction threshold are specified in
  required-model-limits.feature. OpenAIProvider construction fails when the output limit is
  unavailable. Custom providers no longer guess 128000/4096 defaults.
  Split (see spec/attachments/PROV-165/design.md): the session wire fields SessionModel and
  ModelInfo still carry integers; PROV-166 makes them Option. LlmProvider object limits are PROV-167.
  """

  Background: User Story
    As a session owner whose model has no published limits
    I want to have an unavailable context window or output limit reported as unavailable, not as 0
    So that no prompt is admitted, compacted or budgeted against a limit nobody declared

  Scenario: A limit with no user value, catalog value or declared default is unavailable
    Given a model-limits resolver that declares no default context window or output limit
    When its limits are resolved without a user value or a catalog value
    Then the context window is unavailable
    And the output limit is unavailable

  Scenario: A declared provider default still resolves
    Given a Claude provider manager with no catalog or user limits
    When its limits are read
    Then the context window is 200000
    And the output limit is 8192

  Scenario: A custom provider without user or catalog limits has no guessed limits
    Given a custom provider manager with no catalog or user limits
    When its limits are read
    Then the context window is unavailable
    And the output limit is unavailable
