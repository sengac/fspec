@wip
@tui
@model-selection
@persistence
@TUI-035
Feature: Persist Last Used Model Selection

  """
  LAYER ARCHITECTURE:
  1. UI Layer (AgentModal.tsx): Read persisted model on init, update on model switch
  2. Config Layer (src/utils/config.ts): loadConfig(), writeConfig() for ~/.fspec/fspec-config.json
  3. NAPI Layer (codelet-napi): Session creation with newWithModel()

  DATA FLOW - Session Init:
  1. AgentModal mounts, calls loadConfig() to get lastUsedModel
  2. If lastUsedModel exists and provider has credentials, use it as default
  3. Otherwise, fall back to first available model with tool_call=true
  4. Create session with newWithModel(lastUsedModel || defaultModel)

  DATA FLOW - Model Switch:
  1. User presses Tab, selects new model
  2. selectModel() called on session
  3. writeConfig() called to persist new lastUsedModel to user config
  4. Config stored at ~/.fspec/fspec-config.json under "agent.lastUsedModel" key

  FILE STRUCTURE:
  - src/tui/components/AgentModal.tsx - Read/write lastUsedModel config
  - src/utils/config.ts - loadConfig(), writeConfig() utilities
  - ~/.fspec/fspec-config.json - User config storage location

  CRITICAL IMPLEMENTATION REQUIREMENTS:
  - MUST persist to user scope (not project scope) so preference is global
  - MUST validate persisted model exists and provider has credentials before using
  - MUST gracefully handle missing/corrupt config with silent fallback
  - MUST NOT block session creation if config read/write fails
  - Config key path: agent.lastUsedModel (e.g., "anthropic/claude-sonnet-4")
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Model selection persisted to ~/.fspec/fspec-config.json under agent.lastUsedModel key
  #   2. Persistence uses provider/model-id format (e.g., anthropic/claude-sonnet-4)
  #   3. On new session, restore last used model if available and credentials exist
  #   4. Fall back to default model selection if persisted model unavailable
  #   5. Update persistence whenever user switches models via Tab selector
  #
  # EXAMPLES:
  #   1. User selects anthropic/claude-opus-4 via Tab, config.json is updated with lastUsedModel
  #   2. User opens new AgentModal, session starts with previously selected claude-opus-4
  #   3. User's persisted model google/gemini-pro no longer exists, falls back to first available model
  #   4. User's persisted model is anthropic/claude-sonnet-4 but no ANTHROPIC_API_KEY, uses different provider
  #   5. Fresh install with no config.json, uses default model selection logic
  #
  # ========================================

  Background: User Story
    As a developer using fspec's AI agent
    I want to have my last used model remembered
    So that I don't need to re-select my preferred model every time I open the agent

  # ----------------------------------------
  # PERSISTENCE ON MODEL SWITCH
  # ----------------------------------------

  Scenario: Persist model selection when user switches via Tab
    Given I am in the AgentModal with a valid session
    And my current model is "anthropic/claude-sonnet-4"
    When I press Tab to open the model selector
    And I select "anthropic/claude-opus-4"
    Then the session should switch to claude-opus-4
    And ~/.fspec/fspec-config.json should contain "agent.lastUsedModel": "anthropic/claude-opus-4"

  Scenario: Persist model selection across providers
    Given I am in the AgentModal using "anthropic/claude-sonnet-4"
    When I switch to "google/gemini-2.5-pro" via Tab selector
    Then ~/.fspec/fspec-config.json should contain "agent.lastUsedModel": "google/gemini-2.5-pro"

  # ----------------------------------------
  # RESTORATION ON NEW SESSION
  # ----------------------------------------

  Scenario: Restore persisted model on new session
    Given ~/.fspec/fspec-config.json contains "agent.lastUsedModel": "anthropic/claude-opus-4"
    And ANTHROPIC_API_KEY is set
    When I open the AgentModal
    Then the session should start with "anthropic/claude-opus-4"
    And the header should display "Agent: claude-opus-4"

  Scenario: Restore persisted model from different provider
    Given ~/.fspec/fspec-config.json contains "agent.lastUsedModel": "google/gemini-2.5-pro"
    And GOOGLE_GENERATIVE_AI_API_KEY is set
    When I open the AgentModal
    Then the session should start with "google/gemini-2.5-pro"

  # ----------------------------------------
  # FALLBACK SCENARIOS
  # ----------------------------------------

  Scenario: Fall back when persisted model no longer exists
    Given ~/.fspec/fspec-config.json contains "agent.lastUsedModel": "google/old-deprecated-model"
    And GOOGLE_GENERATIVE_AI_API_KEY is set
    When I open the AgentModal
    Then the session should start with the first available model
    And an informational message should indicate the persisted model was unavailable

  Scenario: Fall back when persisted provider has no credentials
    Given ~/.fspec/fspec-config.json contains "agent.lastUsedModel": "anthropic/claude-sonnet-4"
    And ANTHROPIC_API_KEY is NOT set
    And GOOGLE_GENERATIVE_AI_API_KEY is set
    When I open the AgentModal
    Then the session should start with a Google model instead
    And an informational message should indicate the persisted provider was unavailable

  Scenario: Use default selection on fresh install
    Given ~/.fspec/fspec-config.json does not exist
    And ANTHROPIC_API_KEY is set
    When I open the AgentModal
    Then the session should start with the first available model
    And no error should be shown

  Scenario: Handle corrupt config gracefully
    Given ~/.fspec/fspec-config.json contains invalid JSON
    And ANTHROPIC_API_KEY is set
    When I open the AgentModal
    Then the session should start with the first available model
    And config read failure should be logged but not shown to user

  # ----------------------------------------
  # CONFIG STRUCTURE
  # ----------------------------------------

  Scenario: Config uses proper nested structure
    Given I am in the AgentModal
    When I switch to "openai/gpt-4o"
    Then ~/.fspec/fspec-config.json should have structure:
      """
      {
        "agent": {
          "lastUsedModel": "openai/gpt-4o"
        }
      }
      """

  Scenario: Config preserves other settings when updating model
    Given ~/.fspec/fspec-config.json contains other settings like "research.perplexity.apiKey"
    When I switch models to "anthropic/claude-opus-4"
    Then the existing settings should be preserved
    And only "agent.lastUsedModel" should be updated
