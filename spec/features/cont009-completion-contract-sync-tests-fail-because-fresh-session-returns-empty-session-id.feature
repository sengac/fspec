@BUG-155
Feature: cont009_completion_contract_sync tests fail because fresh_session() returns empty session ID
  """
  The fresh_session() helper in rust/sessions/tests/cont009_completion_contract_sync.rs must seed the offline models cache (cache/models.json) before creating the SessionManager, following the pattern established in rpc386_owning_session_manager.rs and prov118_no_session_default_model.rs. Uses prov101_models.json fixture from rust/sessions/tests/fixtures/. Must call reset_stores_for_tests() before set_data_directory() per RPC-423 precedent.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fresh_session() must create a cache/ subdirectory inside the temp data dir before calling set_data_directory()
  #   2. fresh_session() must write the MODELS_FIXTURE content to cache/models.json before creating the SessionManager
  #   3. fresh_session() must call reset_stores_for_tests() before setting the data directory to avoid stale singleton references (RPC-423 precedent)
  #
  # EXAMPLES:
  #   1. fresh_session() creates cache/ directory, writes models.json with prov101_models.json fixture content, calls reset_stores_for_tests(), then sets data directory
  #
  # ========================================
  Background: User Story
    As a developer
    I want to fix the fresh_session() helper to seed the models cache
    So that the cont009_completion_contract_sync tests pass

  Scenario: fresh_session() seeds the models cache so session creation succeeds
    Given the fresh_session() helper in cont009_completion_contract_sync.rs
    When the helper creates a temp data directory
    Then it must create a cache/ subdirectory inside the temp data dir
    And it must write the prov101_models.json fixture content to cache/models.json
    And it must call reset_stores_for_tests() before setting the data directory
    And the subsequent create_session() call must return a valid non-empty session ID
