@done
@tui
@model-selector
@bug-fix
@BUG-207
Feature: The model selector refresh action forces a catalog re-fetch before re-listing providers
  """
  TUI dispatch_model_selector::handle_refresh_model_selector ('r' key) calls backend.refresh_models_cache("") before re-spawning list_providers(); previously it only re-listed, which is a no-op against a stale snapshot.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   5. The model selector's refresh action ('r') must FORCE a catalog re-fetch before re-listing providers; refresh_models_cache(provider_id) force-refreshes the catalog and returns the freshly-sourced entries for that provider (empty provider_id = refresh the shared catalog only)
  #
  # EXAMPLES:
  #   2. A user presses 'r' in the model selector while a months-old snapshot is on disk: the newly released model appears in the list immediately — no restart, no manual cache deletion
  #
  # ========================================
  Background: User Story
    As a user
    I want to see newly released models in the model selector
    So that I can select a model the day it appears in the models.dev catalog without clearing caches or restarting

  # @step comments below are matched verbatim by the tests in
  # rust/fspec-tui/tests/model_selector_refresh_forces_catalog_refetch_bug207.rs
  Scenario: The model selector refresh action forces a catalog re-fetch before re-listing providers
    Given the model selector is open
    When the refresh action is triggered
    Then the models cache is force-refreshed before the provider list is fetched
