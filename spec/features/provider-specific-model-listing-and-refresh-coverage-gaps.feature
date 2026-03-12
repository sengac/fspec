@done
@MODEL-003 @providers @model-selection @cache @critical
Feature: Provider-specific model listing and refresh coverage gaps

  """
  Architecture:
  - models_list_for_provider must apply the same is_current_model() filter and
    newest-first sort as models_list_all() — both public NAPI paths must behave
    identically with respect to deprecated/stale model exclusion.
  - models_refresh_cache() must call invalidate_registry_cache() after updating
    the disk cache. Tests must exercise this exact code path (not simulate it
    manually) so removing the invalidation call causes a test failure.
  - get_registry() must not hold the Mutex guard across async initialization.
    The fix uses a two-phase approach: check under lock → release → initialize →
    re-acquire and store (double-check for concurrent races). This keeps the hot
    path (cached) fast while avoiding unnecessary lock hold time on the cold path.
  """

  Background: User Story
    As a developer
    I want provider-specific model listing to match all-model filtering and refresh behaviour
    So that deprecated and stale models are excluded regardless of which listing API is used

  Scenario: Provider listing filters deprecated and stale models — only current model returned
    Given a provider contains a deprecated model, an older-than-18-months model, and a current model
    When models_list_for_provider is called for that provider
    Then only the current model is returned
    And the deprecated model is excluded
    And the stale model is excluded

  Scenario: Provider listing preserves newest-first ordering
    Given a provider contains two current models with different release dates
    When models_list_for_provider is called for that provider
    Then the newer model appears before the older model in the result

  Scenario: After public refresh behaviour, next listing returns the new model
    Given the registry cache has been initialized with model data containing "codex-5.3"
    When the disk cache is refreshed from codex-5.3 to codex-5.4 through the public refresh behaviour
    And models_list_for_provider is called to rebuild the registry
    Then the returned models contain "codex-5.4"
    And "codex-5.3" is no longer returned

  Scenario: Two concurrent callers and an invalidation remain race-safe
    Given the registry cache is empty
    When two concurrent callers request the registry while another invalidates after refresh
    Then both callers receive a valid registry without a data race
    And subsequent reads rebuild from the refreshed cache
