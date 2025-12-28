@done
@wip
@agent-modal
@model-selection
@tui
@TUI-034
Feature: Agent Modal Model Selection
  """

  LAYER ARCHITECTURE:
  1. UI Layer (AgentModal.tsx): Tab key trigger, hierarchical model selector overlay, header display
  2. State Layer (React): currentModel, providerSections, showModelSelector, expandedProviders
  3. NAPI Layer (codelet-napi): modelsListAll(), CodeletSession.newWithModel(), selectModel(), selectedModel getter
  4. Provider Layer (codelet-providers): ModelRegistry, ModelCache, ProviderManager.select_model()
  5. Cache Layer: ~/.fspec/cache/models.json with embedded fallback

  DATA FLOW - Selector Open:
  1. User presses Tab key in AgentModal
  2. modelsListAll() called to get models grouped by provider
  3. Filter providers to those with credentials (availableProviders intersection)
  4. Filter models to those with tool_call=true capability
  5. Display hierarchical selector with current provider expanded

  DATA FLOW - Model Selection:
  1. User navigates with arrow keys, Enter to select
  2. selectModel("provider/model-id") called on CodeletSession
  3. ProviderManager.select_model() validates and switches model
  4. Header updated to show model name and capability indicators

  FILE STRUCTURE:
  - src/tui/components/AgentModal.tsx - Main modal component, selector overlay
  - codelet/napi/src/session.rs - CodeletSession with newWithModel(), selectModel()
  - codelet/napi/src/models.rs - modelsListAll(), modelsListForProvider(), modelsGetInfo()
  - codelet/providers/src/manager.rs - ProviderManager.select_model(), selected_model_string()
  - codelet/providers/src/models/registry.rs - ModelRegistry validation

  CRITICAL IMPLEMENTATION REQUIREMENTS:
  - MUST use CodeletSession.newWithModel() for session creation (not basic constructor)
  - MUST filter to only models with tool_call=true capability
  - MUST show capability indicators ([R] reasoning, [V] vision, [200k] context)
  - MUST persist full model path "provider/model-id" in session manifest
  - MUST support mid-session model switching via selectModel()
  - MUST gracefully handle missing models with fallback to provider default

  PROVIDER ID MAPPING:
  - models.dev "anthropic" maps to internal "claude"
  - models.dev "google" maps to internal "gemini"
  - models.dev "openai" maps to internal "openai"

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tab key opens model selector (replaces current provider-only selector)
  #   2. Only providers with valid credentials are shown
  #   3. Only models with tool_call=true are selectable (required for agent functionality)
  #   4. Current model should be highlighted with "(current)" indicator
  #   5. Provider sections are collapsible with Left/Right arrow keys
  #   6. Header displays model name and capability indicators, not just provider
  #   7. Session persistence stores full model path for accurate restoration
  #   8. Backward compatible with legacy provider-only session format
  #
  # EXAMPLES:
  #   1. User presses Tab and sees "anthropic" expanded with claude-sonnet-4 highlighted
  #   2. User presses Down to navigate to claude-opus-4, Enter to select
  #   3. Header changes from "Agent: claude" to "Agent: claude-opus-4 [R] [200k]"
  #   4. User opens model selector, presses Right on collapsed "google" to expand it
  #   5. User resumes session with provider "anthropic/claude-sonnet-4" and sees correct model
  #   6. User sees [R] indicator for reasoning models, [V] for vision, context size
  #   7. User only sees "anthropic" section if ANTHROPIC_API_KEY is set
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we keep backward compatibility with provider-only sessions?
  #   A: Yes, legacy sessions with just "claude" should use default model for that provider.
  #
  #   Q: What should happen if selected model no longer exists in cache?
  #   A: Fall back to provider default model and show informational message.
  #
  #   Q: Should Tab key still work if only one provider is available?
  #   A: Yes, user may still want to select different models within that provider.
  #
  # ========================================

  Background: User Story
    As a developer using fspec's AI agent
    I want to select specific models within providers
    So that I can choose the best model for my task and budget

  # ----------------------------------------
  # BASIC SELECTOR BEHAVIOR
  # ----------------------------------------

  Scenario: Tab key opens model selector with providers as collapsible sections
    Given I am in the AgentModal with a valid session
    And multiple providers have valid credentials
    When I press Tab
    Then the model selector overlay should appear
    And I should see available providers as collapsible sections
    And the current provider should be expanded by default
    And the current model should be highlighted with "(current)" indicator

  Scenario: Navigate between provider sections with arrow keys
    Given the model selector is open
    And the "anthropic" provider section is collapsed
    When I press Down arrow to navigate to "google" provider header
    Then the "google" provider header should be highlighted
    And the section should remain collapsed until expanded

  Scenario: Expand provider section with Right arrow or Enter
    Given the model selector is open
    And the "google" provider section is collapsed and highlighted
    When I press Right arrow
    Then the "google" section should expand
    And the first model within "google" should be highlighted

  Scenario: Collapse provider section with Left arrow
    Given the model selector is open
    And I am on a model within the expanded "anthropic" section
    When I press Left arrow
    Then the "anthropic" section should collapse
    And the "anthropic" provider header should be highlighted

  Scenario: Select model with Enter key
    Given the model selector is open
    And "anthropic/claude-opus-4" is highlighted
    When I press Enter
    Then the model selector should close
    And selectModel should be called with "anthropic/claude-opus-4"
    And the header should display the new model name

  Scenario: Cancel model selection with Escape
    Given the model selector is open
    And I have navigated to a different model
    When I press Escape
    Then the model selector should close
    And the original model should remain selected

  # ----------------------------------------
  # CAPABILITY INDICATORS
  # ----------------------------------------

  Scenario: Display reasoning capability indicator
    Given the model selector is open
    When I view a model with reasoning=true
    Then I should see "[R]" indicator next to the model name
    And models with reasoning=false should not show this indicator

  Scenario: Display vision capability indicator
    Given the model selector is open
    When I view a model with hasVision=true
    Then I should see "[V]" indicator next to the model name

  Scenario: Display context window size
    Given the model selector is open
    When I view any model
    Then I should see the context window size formatted as "[200k]" or "[1M]"

  Scenario: Header shows model with capability indicators
    Given the current model is "anthropic/claude-sonnet-4"
    And the model has reasoning=true and contextWindow=200000
    Then the header should display "Agent: claude-sonnet-4 [R] [200k]"

  # ----------------------------------------
  # PROVIDER FILTERING
  # ----------------------------------------

  Scenario: Only show providers with valid credentials
    Given ANTHROPIC_API_KEY is set
    And OPENAI_API_KEY is NOT set
    When I open the model selector
    Then I should see the "anthropic" provider section
    And I should NOT see the "openai" provider section

  Scenario: Only show models with tool_call capability
    Given the model selector is open
    When I view any provider's model list
    Then I should only see models where tool_call=true
    And models without tool_call capability should be hidden

  Scenario: Show message when provider has no compatible models
    Given a provider has only models with tool_call=false
    When I expand that provider section
    Then I should see "No compatible models (tool_call required)"

  # ----------------------------------------
  # SESSION INITIALIZATION
  # ----------------------------------------

  Scenario: New session uses newWithModel factory method
    Given I open the AgentModal
    When the session initializes
    Then CodeletSession.newWithModel should be called
    And the default model should be the first available with tool_call=true

  Scenario: Session stores full model path in persistence
    Given I have selected "anthropic/claude-sonnet-4"
    When I send my first message
    Then the persisted session should store "anthropic/claude-sonnet-4" as the provider field

  Scenario: Resumed session restores exact model
    Given I have a persisted session with provider "anthropic/claude-opus-4"
    When I resume that session via /resume command
    Then selectModel should be called with "anthropic/claude-opus-4"
    And the header should show "Agent: claude-opus-4"

  Scenario: Legacy session with provider-only format uses default model
    Given I have a persisted session with provider "claude" (legacy format)
    When I resume that session
    Then the session should switch to claude provider
    And the default model for claude should be used

  # ----------------------------------------
  # ERROR HANDLING
  # ----------------------------------------

  Scenario: Graceful fallback when model cache unavailable
    Given the models.dev cache is corrupted or unavailable
    When I open the AgentModal
    Then the embedded fallback models should be used
    And model selection should still function

  Scenario: Error message when selected model unavailable
    Given I try to select a model that doesn't exist in the registry
    Then an error message should be displayed in the conversation
    And the current model should remain unchanged

  Scenario: Fallback when resumed session model no longer exists
    Given I have a persisted session with model "anthropic/old-deprecated-model"
    When I resume that session
    Then an informational message should be shown
    And the default model for anthropic should be used instead

  # ----------------------------------------
  # UI DISPLAY FORMAT
  # ----------------------------------------

  Scenario: Provider header shows model count
    Given the "anthropic" provider has 3 models with tool_call=true
    When I view the model selector
    Then the header should show "[anthropic] (3 models)"

  Scenario: Model list shows consistent format
    Given the model selector is open
    Then each model should display in format: "  model-id (Display Name) [indicators]"
    And the selected model should have ">" prefix

  Scenario: Tab hint in header shows model switching available
    Given multiple models are available
    Then the header should show "[Tab] Model" hint
