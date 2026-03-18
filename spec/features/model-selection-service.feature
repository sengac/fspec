@model-selection
@tui
@PROV-008
Feature: Refactor Model Selection Architecture for DRY/SOLID Compliance
  """
  SERVICES:
  - profileEnvironmentService.ts: Configures environment variables for profile-based models
  - modelSelectionService.ts: Orchestrates model selection across session, store, and persistence layers

  DELETION:
  - handleSelectModel callback removed from AgentView.tsx (deprecated, unused)

  INTEGRATION:
  - Services called from AgentView.tsx handleModelSelect
  - Uses NAPI sessionSetModel/sessionSetModelProfile for Rust session updates
  - Persists to user config via writeConfig
  - Updates Zustand store via useModelStore
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Deprecated code paths must be removed before adding new abstractions
  #   2. Environment variable side effects must be isolated in dedicated services, not UI components
  #   3. Model selection service must handle both cloud providers (via registry) and profile-based models (direct)
  #   4. Service extractions must maintain backward compatibility - existing behavior must not change
  #   5. All services must be exported from src/tui/services/index.ts
  #
  # EXAMPLES:
  #   1. Developer deletes handleSelectModel and sees no compilation errors
  #   2. Developer calls configureProfileEnvironment with vLLM profile and OPENAI_BASE_URL is set
  #   3. Developer calls selectModel for cloud provider and session is updated via sessionSetModel
  #   4. Developer calls selectModel for profile model and session is updated via sessionSetModelProfile
  #   5. Developer calls selectModel without session and model is stored for later sync
  #   6. Developer calls selectModel and selection is persisted to config file
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have model selection logic extracted into proper services
    So that reduce code duplication and improve maintainability following DRY/SOLID principles

  Scenario: Delete deprecated handler without breaking build
    Given the AgentView component contains handleSelectModel callback
    When the deprecated handleSelectModel callback is removed
    Then the TypeScript project compiles without errors
    And no code references handleSelectModel

  Scenario: Persist model selection to config file
    Given a model selection for provider "anthropic" model "claude-sonnet-4"
    When selectModel completes successfully
    Then writeConfig should be called with lastUsedModel "anthropic/claude-sonnet-4"

  Scenario: Configure environment variables for profile-based model
    Given a profile config with baseUrl "http://192.168.0.50:8888" and apiKey "test-api-key"
    When configureProfileEnvironment is called with the profile config
    Then OPENAI_BASE_URL should be set to "http://192.168.0.50:8888"
    And OPENAI_API_KEY should be set to "test-api-key"

  Scenario: Select cloud provider model with active session
    Given an active session with id "session-123"
    And a cloud model selection for provider "anthropic" model "claude-sonnet-4"
    When selectModel is called with the session and selection
    Then sessionSetModel should be called with the provider and model
    And the model store should be updated
    And the selection should be persisted to config

  Scenario: Select profile-based model with active session
    Given an active session with id "session-123"
    And a model selection with profileConfig containing baseUrl and apiKey
    When selectModel is called with the session and selection
    Then configureProfileEnvironment should be called with the profileConfig
    And sessionSetModelProfile should be called instead of sessionSetModel

  Scenario: Select model without active session
    Given no active session exists
    And a model selection for provider "openai" model "gpt-4o"
    When selectModel is called with null session and the selection
    Then neither sessionSetModel nor sessionSetModelProfile should be called
    And the model store should be updated for later session sync
    And the selection should be persisted to config
