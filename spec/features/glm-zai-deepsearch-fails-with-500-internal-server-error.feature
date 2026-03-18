@BUG-106
Feature: GLM/ZAI DeepSearch fails with 500 Internal Server Error
  """
  ZAI DeepSearch uses streaming execution and collects the final response. Fix: provider_uses_streaming_execution returns true for both codex and zai. ZAI config includes max_tokens in additional_params.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ZAI DeepSearch must use streaming execution path (like Codex) because Z.AI's non-streaming endpoint fails with 500 on tool-calling requests
  #   2. max_tokens must be included in ZAI additional_params since rig's OpenAI CompletionRequest silently drops it from CoreCompletionRequest
  #   3. provider_uses_streaming_execution() must return true for both 'codex' and 'zai' providers
  #
  # EXAMPLES:
  #   1. DeepSearch with ZAI uses streaming, final response is collected and returned as one String result (same contract as Codex streaming path)
  #   2. ZAI DeepSearch config includes max_tokens: 8192 in additional_params alongside temperature and top_p
  #   3. Non-ZAI/non-Codex providers (claude, openai, gemini) remain non-streaming for DeepSearch
  #
  # ========================================
  Background: User Story
    As a user
    I want to use DeepSearch with the ZAI/GLM provider
    So that explore codebases without hitting 500 errors

  Scenario: ZAI DeepSearch uses streaming execution path
    Given a DeepSearch sub-agent is constructed for provider "zai"
    When the sub-agent executes the query
    Then the execution path uses streaming to collect the final response
    Then the final synthesized answer is returned as one String result

  Scenario: Non-streaming providers remain unchanged
    Given a DeepSearch sub-agent is constructed for provider "claude"
    When the sub-agent executes the query
    Then the execution path remains non-streaming
    Then the final synthesized answer contract remains unchanged

  Scenario: ZAI DeepSearch config includes max_tokens in additional_params
    Given a DeepSearch request config is built for provider "zai"
    When the config is serialized for the HTTP request
    Then the additional_params includes max_tokens set to 8192
    Then the additional_params includes temperature and top_p
