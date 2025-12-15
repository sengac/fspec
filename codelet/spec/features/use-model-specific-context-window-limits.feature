@CLI-015
Feature: Use model-specific context window limits

  """
  Implementation:
  - Add context_window() method to ProviderManager that returns the context window for the current provider type. Each provider already has CONTEXT_WINDOW constants defined (Claude=200k, OpenAI=128k, Gemini=1M, Codex=272k). The method should return usize from ProviderType lookup. In interactive.rs, replace the hardcoded 'const CONTEXT_WINDOW: u64 = 100_000' with session.provider_manager().context_window() as u64. This ensures compaction triggers at 90% of actual context window.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The system MUST maintain a lookup table of model names to their context window sizes
  #   2. For unknown models, the system MUST default to a conservative context window size (e.g., 100,000 tokens)
  #   3. The compaction threshold calculation MUST use the model's actual context window size, not a hardcoded value
  #   4. Model limits MUST include context_window (input limit) and optionally max_output_tokens
  #
  # EXAMPLES:
  #   1. claude-sonnet-4-20250514 has context_window=200000, compaction triggers at 180000 tokens (90%)
  #   2. claude-3-5-sonnet-20241022 has context_window=200000, compaction triggers at 180000 tokens
  #   3. gpt-4o has context_window=128000, compaction triggers at 115200 tokens (90%)
  #   4. unknown-model-xyz defaults to context_window=100000, compaction triggers at 90000 tokens
  #
  # ========================================

  Background: User Story
    As a AI agent system
    I want to use model-specific context window limits for compaction threshold calculations
    So that compaction triggers at the correct 90% of actual capacity instead of incorrect percentages based on hardcoded values


  Scenario: Claude provider returns correct context window
    Given the current provider is Claude
    When I query the context window size
    Then the context window should be 200000 tokens


  Scenario: OpenAI provider returns correct context window
    Given the current provider is OpenAI
    When I query the context window size
    Then the context window should be 128000 tokens


  Scenario: Gemini provider returns correct context window
    Given the current provider is Gemini
    When I query the context window size
    Then the context window should be 1000000 tokens


  Scenario: Codex provider returns correct context window
    Given the current provider is Codex
    When I query the context window size
    Then the context window should be 272000 tokens


  Scenario: Compaction threshold uses model-specific context window
    Given a Claude provider with context_window=200000
    And effective_tokens has reached 180000 (90% of context window)
    When the compaction check is performed
    Then compaction should be triggered


  Scenario: Compaction does not trigger below threshold
    Given a Claude provider with context_window=200000
    And effective_tokens is at 170000 (85% of context window)
    When the compaction check is performed
    Then compaction should NOT be triggered
