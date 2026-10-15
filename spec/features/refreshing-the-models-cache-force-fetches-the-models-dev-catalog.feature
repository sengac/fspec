@done
@providers
@session
@model-selection
@bug-fix
@BUG-207
Feature: Refreshing the models cache force-fetches the models.dev catalog
  """
  sessions handle_impl::refresh_models_cache(provider_id) FORCE-fetches the models.dev catalog (ModelRegistry::refreshing via the private force_refresh_cloud_registry helper) and returns the freshly-sourced entries for that provider via cloud_models::cloud_model_entries — previously it re-read the same on-disk snapshot and returned an EMPTY list for every built-in provider. An empty provider_id means 'refresh the shared catalog only' (the model selector's refresh spans all providers). Custom providers keep their statically-declared list as fallback.
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
  # rust/sessions/tests/bug207_models_catalog_force_refresh.rs
  Scenario: Refreshing the models cache force-fetches the catalog and returns the provider's models
    Given the models.dev snapshot on disk is months old and lacks a newly released model for a credentialed built-in provider
    When the models cache is refreshed for that provider
    Then the catalog is re-fetched from models.dev
    And the returned model entries include the newly released model

  Scenario: Refreshing with no provider refreshes the shared catalog only
    Given the models.dev snapshot on disk is months old
    When the models cache is refreshed with an empty provider id
    Then the catalog is re-fetched from models.dev
    And no provider-specific model entries are returned
