@done
@providers
@caching
@model-selection
@bug-fix
@BUG-207
Feature: Models.dev catalog cache serves stale snapshots while revalidating in the background
  """
  rust/providers/src/models/cache.rs: ModelCache now has a TTL (DEFAULT_CACHE_TTL = 24h) with stale-while-revalidate. get() serves the on-disk snapshot when fresh; when stale it serves it immediately and spawns a background revalidation (REVALIDATE_TIMEOUT 10s) throttled by a process-wide AtomicU64 cooldown (REVALIDATE_COOLDOWN 5min) because sessions' build_cloud_registry() constructs a NEW ModelCache per list_providers() call. Missing/corrupt cache still fetches synchronously (FETCH_TIMEOUT 30s). refresh() bypasses TTL + cooldown. ModelRegistry::refreshing(&cache) is the force-refresh constructor.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A models.dev snapshot is fresh for at most DEFAULT_CACHE_TTL (24h); a snapshot older than the TTL must never keep being served unchanged
  #   2. Stale-while-revalidate: a stale snapshot is served IMMEDIATELY (no UI blocking) and a revalidation is kicked off; the next read sees the new data
  #   3. Revalidation attempts are throttled process-wide (5-minute cooldown) because every list_providers() call builds a new ModelCache; the throttle protects the network, not correctness
  #   4. An explicit refresh (ModelCache::refresh / ModelRegistry::refreshing) is a SYNCHRONOUS fetch that bypasses both the TTL and the cooldown — it must actually hit the network
  #
  # EXAMPLES:
  #   1. A models.dev snapshot on disk is 30 days old and contains no gpt-6; opening the model selector serves the old snapshot immediately AND revalidates in the background; the next open lists gpt-6
  #   3. A snapshot written minutes ago is fresh: opening the model selector performs no network request at all
  #   4. The catalog endpoint is unreachable and the snapshot is stale: the stale list is still served (never an empty selector) and the failure is only logged
  #
  # ========================================
  Background: User Story
    As a user
    I want to see newly released models in the model selector
    So that I can select a model the day it appears in the models.dev catalog without clearing caches or restarting

  # @step comments below are matched verbatim by the tests in
  # rust/providers/src/models/cache.rs (inline module)
  Scenario: A stale models.dev snapshot is revalidated instead of served forever
    Given the models.dev snapshot on disk is older than the cache TTL
    When the model catalog is read
    Then the stale snapshot is served immediately
    And a revalidation of the catalog is attempted

  Scenario: A fresh models.dev snapshot is served without any network request
    Given the models.dev snapshot on disk is younger than the cache TTL
    When the model catalog is read
    Then the snapshot is served from disk
    And no revalidation of the catalog is attempted

  Scenario: Revalidation attempts are throttled
    Given the models.dev snapshot on disk is older than the cache TTL
    When the model catalog is revalidated twice in a row
    Then only the first revalidation reaches the network

  Scenario: An unreachable catalog never empties the model selector
    Given the models.dev snapshot on disk is older than the cache TTL
    And the catalog endpoint is unreachable
    When the model catalog is read
    Then the stale snapshot is still served
    And the failure is only logged
