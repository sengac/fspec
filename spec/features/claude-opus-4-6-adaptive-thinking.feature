@done
@provider-abstraction
@high
@codelet
@providers
@PROV-005
Feature: Add Claude Opus 4.6 Support with Adaptive Thinking

  """
  ARCHITECTURE:
  - Follow VTCode/OpenCode approach: simple enum variant + model check, no separate facade needed
  - Use EXPLICIT model constants (CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6) with exact equality (==)
  - NO pattern matching (.contains(), .starts_with()) - prevents accidental matches
  - Group models by capability in explicit lists: ADAPTIVE_THINKING_MODELS, CONTEXT_1M_MODELS
  - Versioned models (claude-opus-4-6-20260201) must be explicitly added when released
  
  OFFICIAL ANTHROPIC SPEC (from platform.claude.com/docs):
  
  ADAPTIVE THINKING MODELS (type: "adaptive", NO interleaved-thinking header needed):
  - claude-opus-4-6   ✓ adaptive ✓ context-1m
  - claude-sonnet-4-6 ✓ adaptive ✓ context-1m
  
  BUDGETED THINKING MODELS (type: "enabled" + budget_tokens, NEED interleaved-thinking header):
  - claude-sonnet-4-5 ✓ interleaved-thinking ✓ context-1m
  - claude-opus-4-5   ✓ interleaved-thinking ✗ context-1m (NO 1M support)
  - older/unknown     ✓ interleaved-thinking ✗ context-1m
  
  BETA HEADERS:
  - prompt-caching-2024-07-31:       ALL models
  - interleaved-thinking-2025-05-14: ONLY non-adaptive models (4.6 models don't need it)
  - context-1m-2025-08-07:           Opus 4.6, Sonnet 4.6, Sonnet 4.5
  
  NOTE: output-64k-2025-02-19 was removed as it's no longer a valid beta header.
  64K/128K output is now standard based on model (set via max_tokens parameter).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   [0] Opus 4.6 and Sonnet 4.6 MUST use adaptive thinking (type: adaptive)
  #   [1] User-provided budget_tokens MUST be ignored for adaptive thinking models
  #   [2] ThinkingConfig enum MUST have Adaptive variant that serializes to {type: adaptive}
  #   [4] Non-adaptive models MUST continue using budget-based thinking (type: enabled, budget_tokens: N)
  #   [5] Adaptive thinking models MUST NOT include interleaved-thinking header (automatic)
  #   [6] Opus 4.6, Sonnet 4.6, Sonnet 4.5 MUST include context-1m-2025-08-07 header
  #   [7] Model detection MUST use explicit string constants and exact equality (==)
  #   [8] New model versions MUST be added explicitly to capability lists when released
  #   [9] Opus 4.5 does NOT support 1M context - must NOT include context-1m header
  #   [10] /thinking low/med/high defaults to adaptive for 4.6 models; only /thinking off disables
  #
  # ARCHITECTURE NOTES:
  #   [0] Follow VTCode/OpenCode approach: simple enum variant + model check
  #   [1] Beta headers are model-specific based on official Anthropic documentation
  #   [2] Use explicit model constants with exact equality checks - no pattern matching
  #   [3] Group models by capability: ADAPTIVE_THINKING_MODELS, CONTEXT_1M_MODELS
  #   [4] Versioned models must be explicitly added to capability lists when released
  #
  # ========================================

  Background: User Story
    As a developer using Claude models
    I want to use Claude Opus 4.6 and Sonnet 4.6 with adaptive thinking
    So that the model automatically decides thinking depth without manual budget configuration

  # ===========================================
  # ADAPTIVE THINKING SCENARIOS (Opus 4.6, Sonnet 4.6)
  # ===========================================

  @adaptive-thinking
  Scenario: Opus 4.6 uses adaptive thinking automatically
    Given I have configured the Claude provider with model "claude-opus-4-6"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  @adaptive-thinking
  Scenario: Sonnet 4.6 uses adaptive thinking automatically
    Given I have configured the Claude provider with model "claude-sonnet-4-6"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  @adaptive-thinking @budget-ignored
  Scenario: User-provided budget_tokens is ignored for Opus 4.6
    Given I have configured the Claude provider with model "claude-opus-4-6"
    And I have set a thinking budget of 16000 tokens
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  @adaptive-thinking @budget-ignored
  Scenario: User-provided budget_tokens is ignored for Sonnet 4.6
    Given I have configured the Claude provider with model "claude-sonnet-4-6"
    And I have set a thinking budget of 16000 tokens
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  # ===========================================
  # BUDGETED THINKING SCENARIOS (Opus 4.5, Sonnet 4.5, etc.)
  # ===========================================

  @budget-thinking
  Scenario: Opus 4.5 uses budget-based thinking
    Given I have configured the Claude provider with model "claude-opus-4-5"
    And I have set a thinking budget of 16000 tokens
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "enabled"
    And the request should contain budget_tokens of 16000

  @budget-thinking
  Scenario: Sonnet 4.5 uses budget-based thinking
    Given I have configured the Claude provider with model "claude-sonnet-4-5"
    And I have set a thinking budget of 16000 tokens
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "enabled"
    And the request should contain budget_tokens of 16000

  # ===========================================
  # BETA HEADER SCENARIOS - ADAPTIVE MODELS
  # ===========================================

  @beta-headers @adaptive-thinking
  Scenario: Opus 4.6 uses correct beta headers
    Given I have configured the Claude provider with model "claude-opus-4-6"
    When I make an API request
    Then the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the anthropic-beta header should include "context-1m-2025-08-07"
    And the anthropic-beta header should NOT include "interleaved-thinking-2025-05-14"

  @beta-headers @adaptive-thinking
  Scenario: Sonnet 4.6 uses correct beta headers
    Given I have configured the Claude provider with model "claude-sonnet-4-6"
    When I make an API request
    Then the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the anthropic-beta header should include "context-1m-2025-08-07"
    And the anthropic-beta header should NOT include "interleaved-thinking-2025-05-14"

  # ===========================================
  # BETA HEADER SCENARIOS - BUDGETED MODELS
  # ===========================================

  @beta-headers @budget-thinking
  Scenario: Opus 4.5 uses correct beta headers without 1M context
    Given I have configured the Claude provider with model "claude-opus-4-5"
    When I make an API request
    Then the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
    And the anthropic-beta header should NOT include "context-1m-2025-08-07"

  @beta-headers @budget-thinking
  Scenario: Sonnet 4.5 uses correct beta headers with 1M context
    Given I have configured the Claude provider with model "claude-sonnet-4-5"
    When I make an API request
    Then the anthropic-beta header should include "prompt-caching-2024-07-31"
    And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
    And the anthropic-beta header should include "context-1m-2025-08-07"

  # ===========================================
  # EXPLICIT MODEL CONSTANT SCENARIOS
  # ===========================================

  @model-detection @explicit-constants
  Scenario: Unknown model uses default behavior with explicit constant matching
    Given I have configured the Claude provider with model "claude-opus-4-7"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "enabled"
    And the request should contain a budget_tokens field
    And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
    And the anthropic-beta header should NOT include "context-1m-2025-08-07"

  @model-detection @explicit-constants
  Scenario: Partial model name does not match adaptive thinking models
    Given I have configured the Claude provider with model "claude-opus-4-6-preview"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "enabled"
    And the request should contain a budget_tokens field
    And the anthropic-beta header should NOT include "context-1m-2025-08-07"

  # ===========================================
  # THINKING LEVEL SCENARIOS (low/med/high → adaptive, off → disabled)
  # ===========================================

  @thinking-levels @adaptive-thinking
  Scenario: Thinking level 'high' defaults to adaptive for Opus 4.6
    Given I have configured the Claude provider with model "claude-opus-4-6"
    And I have set the thinking level to "high"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  @thinking-levels @adaptive-thinking
  Scenario: Thinking level 'low' defaults to adaptive for Sonnet 4.6
    Given I have configured the Claude provider with model "claude-sonnet-4-6"
    And I have set the thinking level to "low"
    When I make an API request with thinking enabled
    Then the request should contain thinking configuration with type "adaptive"
    And the request should NOT contain a budget_tokens field

  @thinking-levels @adaptive-thinking
  Scenario: Thinking disabled with 'off' for Opus 4.6
    Given I have configured the Claude provider with model "claude-opus-4-6"
    And I have set the thinking level to "off"
    When I make an API request
    Then the request should NOT contain a thinking configuration

  @thinking-levels @adaptive-thinking
  Scenario: Thinking disabled with 'off' for Sonnet 4.6
    Given I have configured the Claude provider with model "claude-sonnet-4-6"
    And I have set the thinking level to "off"
    When I make an API request
    Then the request should NOT contain a thinking configuration

