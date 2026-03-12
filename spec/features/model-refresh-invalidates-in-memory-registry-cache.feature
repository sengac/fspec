@MODEL-002
Feature: Model refresh does not invalidate in-memory registry cache

  """
  REGISTRY_CACHE uses Mutex<Option<Arc<ModelRegistry>>> instead of OnceCell. models_refresh_cache() calls invalidate_registry_cache() after writing fresh data to disk. get_registry() lazily initializes on first call and returns cached Arc on subsequent calls.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. REGISTRY_CACHE used tokio::sync::OnceCell which can only be initialized once and never reset
  #   2. models_refresh_cache() must invalidate the in-memory registry cache after writing fresh data to disk
  #   3. After invalidation, models_list_all() must rebuild the registry from the refreshed disk cache
  #   4. The registry cache must remain lazy-initialized (don't load on startup if not needed)
  #   5. Concurrent calls to get_registry() during/after invalidation must be safe (no data race)
  #
  # EXAMPLES:
  #   1. models.dev adds Codex 5.4, user presses 'r', cache refreshes, models_list_all() still returns old list with only Codex 5.3 — because OnceCell is never reset
  #   2. After replacing OnceCell with Mutex<Option<...>> and calling invalidate on refresh, models_list_all() returns the fresh registry with new models
  #   3. First call to get_registry() initializes the cache, second call returns the cached value without re-parsing (lazy init preserved)
  #
  # ========================================

  Background: User Story
    As a developer
    I want to refresh the model list in the TUI
    So that see newly available models without restarting the application

  Scenario: Registry returns stale data when OnceCell is not invalidated after refresh
    Given the REGISTRY_CACHE has been initialized with model data containing "codex-5.3"
    And the disk cache has been refreshed with new data containing "codex-5.4"
    When models_list_all is called without invalidating the in-memory cache
    Then the returned models still contain "codex-5.3" but not "codex-5.4"

  Scenario: Refresh invalidates in-memory cache so new models appear
    Given the REGISTRY_CACHE has been initialized with model data containing "codex-5.3"
    When models_refresh_cache is called which fetches fresh data and invalidates the in-memory cache
    And models_list_all is called to rebuild the registry
    Then the returned models contain "codex-5.4"

  Scenario: Registry is lazy-initialized and cached across calls
    Given the REGISTRY_CACHE is empty (no prior initialization)
    When get_registry is called for the first time
    Then it initializes the registry from the disk cache
    And a second call returns the same cached Arc without re-parsing

