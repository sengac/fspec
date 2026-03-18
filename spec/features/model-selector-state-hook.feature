@model-selector
@tui-component
@TUI-072
Feature: Create useModelSelectorState hook
  """
  Hook follows useProviderSettingsState pattern. Uses useState for all state, useCallback for operations, useMemo for computed values (flatItems, filteredFlatItems). Loads models from NAPI (modelsListAll for cloud, modelsListLocalOpenai for profiles). Types from src/tui/types/provider.ts. Provider ID mappings: anthropic↔claude, google↔gemini. Model persistence via buildModelString/parseModelString from model-selection.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Hook must follow the useProviderSettingsState pattern: useState for state, useCallback for operations, useMemo for computed values
  #   2. Hook must expose all model selector state: currentModel, providerSections, selectedSectionIdx, selectedModelIdx, expandedProviders, scrollOffset, filter, isFilterMode, isRefreshing
  #   3. Hook must provide flatItems computed value built from providerSections and expandedProviders
  #   4. Hook must load models on mount using modelsListAll() and modelsListLocalOpenai() NAPI functions
  #   5. Hook must provide navigation helpers: navigateUp(), navigateDown(), getCurrentFlatIndex()
  #   6. Hook must provide toggleSectionExpansion(providerId) for collapsing/expanding provider sections
  #   7. Hook must provide refreshModels() that calls modelsRefreshCache() and reloads
  #   8. Types (ModelSelection, ProviderSection, ModelSelectorItem) must be imported from src/tui/types/provider.ts (consolidated in TUI-076)
  #   9. Hook must expose visibility state (isVisible/showModelSelector) for controlling when selector is shown
  #   10. Hook must expose modelsInitialized boolean to track if initial model load completed
  #   11. Hook must expose isFilterMode for controlling filter input mode
  #   12. Hook must provide filteredFlatItems computed from flatItems filtered by the filter string (case-insensitive match on provider name, model ID, or model name)
  #   13. Hook must auto-scroll to keep selection visible: if selectedFlatIdx < scrollOffset, set scrollOffset to selectedFlatIdx; if selectedFlatIdx >= scrollOffset + visibleHeight, set scrollOffset to selectedFlatIdx - visibleHeight + 1
  #   14. Hook must reset scroll/filter when model selector opens: scrollOffset=0, filter='', isFilterMode=false
  #   15. Hook must reset selection to first item when filter changes: if filteredFlatItems not empty, set selection to first item and scrollOffset to 0
  #   16. Hook must load both cloud providers (from modelsListAll) and profile sections (from loadProviderProfiles + modelsListLocalOpenai) on mount
  #   17. Hook must provide selectModel function that returns ModelSelection with providerId, modelId, apiModelId, displayName, reasoning, hasVision, contextWindow, maxOutput, profileName, profileConfig
  #
  # EXAMPLES:
  #   1. Hook initializes with isLoading=true, then loads models from NAPI and sets isLoading=false
  #   2. When Anthropic section is collapsed and user calls toggleSectionExpansion('anthropic'), expandedProviders Set adds 'anthropic' and flatItems includes Anthropic models
  #   3. navigateDown() from section header (modelIdx=-1) moves to first model in expanded section (modelIdx=0)
  #   4. navigateUp() from first model in section skips collapsed sections and lands on previous expanded section's last model
  #   5. refreshModels() sets isRefreshing=true, calls modelsRefreshCache(), reloads from NAPI, then sets isRefreshing=false
  #   6. setFilter('claude') filters flatItems to only show sections/models matching 'claude' (case-insensitive)
  #   7. selectModel(section, model) returns ModelSelection with providerId, modelId, displayName extracted correctly
  #   8. When model selector opens (isVisible changes to true), scrollOffset resets to 0, filter to empty, and isFilterMode to false
  #   9. Profile sections from local servers (vLLM/Ollama) are loaded using modelsListLocalOpenai and merged with cloud provider sections
  #   10. When filter changes and results exist, selection jumps to first item in filteredFlatItems and scrollOffset resets to 0
  #
  # ========================================
  Background: User Story
    As a TUI developer
    I want to use a dedicated useModelSelectorState hook
    So that model selector state is decoupled from AgentView and can be tested independently

  # ============================================================================
  # INITIALIZATION AND LOADING
  # ============================================================================
  Scenario: Hook initializes with loading state and loads cloud models
    Given I render a component using useModelSelectorState
    When the hook initializes
    Then isLoading should be true initially
    And modelsListAll should be called to fetch cloud provider models
    And isLoading should become false after loading completes
    And modelsInitialized should become true

  Scenario: Hook loads profile sections from local servers
    Given I render a component using useModelSelectorState
    And there are configured profiles for providers
    When the hook initializes
    Then loadProviderProfiles should be called for each SUPPORTED_PROVIDERS
    And modelsListLocalOpenai should be called for each profile's baseUrl
    And profile sections should be merged with cloud sections in providerSections

  Scenario: Hook handles unreachable local servers gracefully
    Given I render a component using useModelSelectorState
    And a profile has an unreachable baseUrl
    When the hook initializes
    Then the profile section should be marked with isUnreachable=true
    And the profile should still appear in providerSections with empty models

  # ============================================================================
  # FLAT LIST COMPUTATION
  # ============================================================================
  Scenario: flatItems is built from providerSections and expandedProviders
    Given a useModelSelectorState hook with loaded models
    And providerSections contains Anthropic with 3 models and OpenAI with 2 models
    When expandedProviders contains "anthropic" but not "openai"
    Then flatItems should contain a section item for Anthropic
    And flatItems should contain 3 model items for Anthropic
    And flatItems should contain a section item for OpenAI
    And flatItems should NOT contain model items for OpenAI

  Scenario: filteredFlatItems filters by provider name case-insensitively
    Given a useModelSelectorState hook with multiple providers
    And providerSections contains Anthropic and OpenAI
    When filter is set to "ANTHRO"
    Then filteredFlatItems should only contain Anthropic section and models
    And OpenAI section should not appear in filteredFlatItems

  Scenario: filteredFlatItems filters by model name or ID
    Given a useModelSelectorState hook with loaded models
    And Anthropic section contains claude-sonnet-4 and claude-opus-4
    When filter is set to "sonnet"
    Then filteredFlatItems should contain Anthropic section
    And filteredFlatItems should contain only claude-sonnet-4 model

  # ============================================================================
  # SECTION EXPANSION
  # ============================================================================
  Scenario: Toggle section expansion adds provider to expanded set
    Given a useModelSelectorState hook with collapsed Anthropic section
    And expandedProviders does not contain "anthropic"
    When I call toggleSectionExpansion with "anthropic"
    Then expandedProviders should contain "anthropic"
    And flatItems should include Anthropic models

  Scenario: Toggle section expansion removes provider from expanded set
    Given a useModelSelectorState hook with expanded Anthropic section
    And expandedProviders contains "anthropic"
    When I call toggleSectionExpansion with "anthropic"
    Then expandedProviders should not contain "anthropic"
    And flatItems should not include Anthropic models

  # ============================================================================
  # NAVIGATION
  # ============================================================================
  Scenario: Navigate down from section header to first model
    Given a useModelSelectorState hook with an expanded section
    And selectedSectionIdx is 0 and selectedModelIdx is -1
    When I call navigateDown
    Then selectedModelIdx should become 0
    And the first model in the section should be selected

  Scenario: Navigate down within models in expanded section
    Given a useModelSelectorState hook with an expanded section containing 3 models
    And selectedModelIdx is 0
    When I call navigateDown
    Then selectedModelIdx should become 1

  Scenario: Navigate down from last model to next section
    Given a useModelSelectorState hook with two sections
    And selectedSectionIdx is 0 and selectedModelIdx is the last model
    When I call navigateDown
    Then selectedSectionIdx should become 1
    And selectedModelIdx should become -1

  Scenario: Navigate up from first model to section header
    Given a useModelSelectorState hook with an expanded section
    And selectedModelIdx is 0
    When I call navigateUp
    Then selectedModelIdx should become -1

  Scenario: Navigate up skips collapsed sections
    Given a useModelSelectorState hook with three sections
    And section 0 is expanded, section 1 is collapsed, section 2 is expanded
    And selected position is section 2 header
    When I call navigateUp
    Then selection should skip section 1
    And selection should land on section 0's last model

  Scenario: Navigate up from section header to previous section's last model
    Given a useModelSelectorState hook with two expanded sections
    And section 0 has 3 models and section 1 is selected at header
    When I call navigateUp
    Then selectedSectionIdx should become 0
    And selectedModelIdx should become 2

  Scenario: getCurrentFlatIndex returns correct position
    Given a useModelSelectorState hook with expanded sections
    And selectedSectionIdx is 1 and selectedModelIdx is 2
    When I call getCurrentFlatIndex
    Then it should return the flat index matching section 1 model 2

  # ============================================================================
  # SCROLL MANAGEMENT
  # ============================================================================
  Scenario: Auto-scroll keeps selection visible when moving down
    Given a useModelSelectorState hook with 20 items
    And visibleHeight is 10 and scrollOffset is 0
    When selection moves to flat index 12
    Then scrollOffset should adjust to 3 to keep selection visible

  Scenario: Auto-scroll keeps selection visible when moving up
    Given a useModelSelectorState hook with 20 items
    And visibleHeight is 10 and scrollOffset is 10
    When selection moves to flat index 5
    Then scrollOffset should adjust to 5 to keep selection visible

  Scenario: Scroll and filter reset when model selector opens
    Given a useModelSelectorState hook with filter="test" and scrollOffset=5
    When isVisible changes from false to true
    Then scrollOffset should reset to 0
    And filter should reset to empty string
    And isFilterMode should reset to false

  # ============================================================================
  # FILTER BEHAVIOR
  # ============================================================================
  Scenario: Filter change resets selection to first result
    Given a useModelSelectorState hook with multiple providers
    And selection is at section 2 model 3
    When filter changes to "openai"
    Then selection should move to first item in filteredFlatItems
    And scrollOffset should reset to 0

  Scenario: Filter matches provider ID and model name
    Given a useModelSelectorState hook with providers and models
    When filter is set to "gpt"
    Then filteredFlatItems should contain OpenAI section
    And filteredFlatItems should contain models with "gpt" in name or ID

  # ============================================================================
  # REFRESH MODELS
  # ============================================================================
  Scenario: Refresh models updates cache and reloads all models
    Given a useModelSelectorState hook with loaded models
    When I call refreshModels
    Then isRefreshing should become true
    And modelsRefreshCache should be called
    And modelsListAll should be called to reload models
    And providerSections should be updated with new models
    And isRefreshing should become false

  # ============================================================================
  # MODEL SELECTION
  # ============================================================================
  Scenario: Select cloud provider model returns complete ModelSelection
    Given a useModelSelectorState hook with loaded models
    And Anthropic section has claude-sonnet-4 model
    When I call selectModel with Anthropic section and claude-sonnet-4 model
    Then the returned ModelSelection should have providerId "anthropic"
    And the returned ModelSelection should have modelId extracted correctly
    And the returned ModelSelection should have apiModelId from the model
    And the returned ModelSelection should have displayName from model.name
    And the returned ModelSelection should have reasoning flag from model
    And the returned ModelSelection should have hasVision flag from model
    And the returned ModelSelection should have contextWindow from model
    And the returned ModelSelection should have maxOutput from model

  Scenario: Select profile model includes profile configuration
    Given a useModelSelectorState hook with a profile section
    And the profile section has profileName "work-vllm" and profileConfig
    When I call selectModel with the profile section and a model
    Then the returned ModelSelection should have profileName "work-vllm"
    And the returned ModelSelection should have profileConfig with baseUrl and apiKey
