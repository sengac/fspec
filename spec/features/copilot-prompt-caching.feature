@PROV-058
Feature: Add prompt caching for GitHub Copilot provider connections
  """
  Inject cache control in CopilotHttpClient::send() by parsing the JSON body, checking the model field, and adding copilot_cache_control to the right positions before re-serializing. New pure function: inject_copilot_cache_control(body: &mut Value, model: &str)
  The body is already parsed in classify_body() — extend that path to also inject cache control, avoiding a double parse/serialize
  New file: copilot/prompt_cache.rs — pure function module for cache control injection, testable independently
  Response-side cached_tokens tracking is handled automatically by rig-core's OpenAI completion parser (prompt_tokens_details.cached_tokens → Usage::cache_read_input_tokens) — no Copilot-specific parsing needed since the Copilot proxy uses standard OpenAI response format.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. copilot_cache_control: {type: 'ephemeral'} must be set on the system message when the model is a Claude-family model (starts with 'claude-')
  #   2. copilot_cache_control: {type: 'ephemeral'} must be set on the last tool definition in the tools array
  #   3. copilot_cache_control: {type: 'ephemeral'} must be set on the last non-user message in the conversation (the cache breakpoint before the final user turn)
  #   4. Cache control must ONLY be applied for Claude-family models (model_id starts with 'claude-'). GPT and Gemini models on Copilot do NOT use copilot_cache_control.
  #   5. Injection must happen in CopilotHttpClient middleware by post-processing the JSON body — not by modifying rig's Message/ToolDefinition types
  #   6. cached_tokens from usage.prompt_tokens_details.cached_tokens in the response should be parsed and propagated to the TUI token display
  #   7. The model name must be extracted from the request body's 'model' field to determine if it's a Claude model — the middleware has no other context about which model is being used
  #
  # EXAMPLES:
  #   1. Claude model via Copilot with 3-turn conversation: system msg gets copilot_cache_control, last tool gets it, last assistant message before the final user turn gets it — cached_tokens reported in usage
  #   2. GPT-5 model via Copilot: no copilot_cache_control fields appear on any messages or tools — request body unchanged
  #   3. Single-turn conversation (only system + user): system gets copilot_cache_control, no other messages need it since there's no prior assistant message
  #   4. Empty tools array: no tool gets copilot_cache_control since there's nothing to tag
  #
  # ========================================
  Background: User Story
    As a developer using fspec with a Copilot subscription
    I want to have prompt caching automatically applied on Copilot API calls
    So that my multi-turn conversations get faster responses and use fewer billed tokens

  @claude
  @multi-turn
  Scenario: Claude model multi-turn conversation gets cache control on system, last tool, and last assistant message
    Given a Copilot API request body with model "claude-sonnet-4"
    And the request has a system message, 3 conversation turns, and 5 tool definitions
    When the CopilotHttpClient middleware processes the request
    Then the system message should have copilot_cache_control set to ephemeral
    And the last tool definition should have copilot_cache_control set to ephemeral
    And the last assistant message before the final user turn should have copilot_cache_control set to ephemeral
    And no other messages or tools should have copilot_cache_control

  @gpt
  @negative
  Scenario: GPT model requests are not modified with cache control
    Given a Copilot API request body with model "gpt-5"
    And the request has a system message, 3 conversation turns, and 5 tool definitions
    When the CopilotHttpClient middleware processes the request
    Then no messages should have copilot_cache_control
    And no tools should have copilot_cache_control

  @gemini
  @negative
  Scenario: Gemini model requests are not modified with cache control
    Given a Copilot API request body with model "gemini-2.5-pro"
    And the request has a system message and 2 conversation turns
    When the CopilotHttpClient middleware processes the request
    Then no messages should have copilot_cache_control

  @claude
  @single-turn
  Scenario: Single-turn Claude conversation only caches system message
    Given a Copilot API request body with model "claude-sonnet-4.5"
    And the request has only a system message and one user message
    When the CopilotHttpClient middleware processes the request
    Then the system message should have copilot_cache_control set to ephemeral
    And the user message should not have copilot_cache_control

  @claude
  @edge-case
  Scenario: Claude request with empty tools array does not crash
    Given a Copilot API request body with model "claude-opus-4.5"
    And the request has a system message, 2 conversation turns, and no tools
    When the CopilotHttpClient middleware processes the request
    Then the system message should have copilot_cache_control set to ephemeral
    And the last assistant message should have copilot_cache_control set to ephemeral
    And no error should occur from the empty tools array
