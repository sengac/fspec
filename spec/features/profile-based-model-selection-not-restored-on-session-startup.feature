@done
@BUG-097
Feature: Profile-based model selection not restored on session startup
  """
  Broken code in modelInitializationService.ts lines 318-325 uses split('/'). Replace with parseModelString() and findSectionForPersistedModel() from model-selection.ts
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. modelInitializationService.ts must use parseModelString() from model-selection.ts instead of simple split('/')
  #   2. Profile section lookup must match by BOTH providerId AND profileName, not just providerId
  #   3. Use existing findSectionForPersistedModel() from model-selection.ts instead of broken findModelInSections()
  #
  # EXAMPLES:
  #   1. Profile model 'openai:qwen3-coder-next/Qwen/Qwen3-Next-80B' → providerId='openai', profileName='qwen3-coder-next', modelId='Qwen/Qwen3-Next-80B'
  #   2. Cloud model 'anthropic/claude-sonnet-4' → providerId='anthropic', profileName=null, modelId='claude-sonnet-4'
  #   3. User selects Qwen via local vLLM, closes fspec, reopens → should see Qwen selected, not fallback to first model
  #
  # ========================================
  Background: User Story
    As a developer using fspec's AI agent
    I want to have my profile-based model selection remembered across sessions
    So that I don't need to re-select my local vLLM model every time I restart fspec

  # ----------------------------------------
  # PROFILE MODEL RESTORATION
  # ----------------------------------------
  Scenario: Restore persisted profile-based model on new session
    Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "openai:work-vllm/Qwen/Qwen3-80B"
    And I have a profile "work-vllm" configured for "openai" provider
    And the profile's local server is reachable
    When I call initializeModels()
    Then the restored model should have providerId="openai"
    And the restored model should have profileName="work-vllm"
    And the restored model should have modelId containing "Qwen"
    And persistedModelRestored should be true

  Scenario: Restore persisted cloud model on new session
    Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "anthropic/claude-sonnet-4"
    And I have credentials for anthropic
    When I call initializeModels()
    Then the restored model should have providerId="anthropic"
    And the restored model should have profileName=null
    And the restored model should have modelId="claude-sonnet-4"
    And persistedModelRestored should be true
