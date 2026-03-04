@wip
@providers
@llm-provider
@PROV-037
Feature: Codex Provider Reasoning Configuration

  """
  Key files: codelet/providers/src/codex/mod.rs (create_rig_agent + complete_with_tools),
  codelet/napi/src/thinking_config.rs (get_thinking_config),
  codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs (AdditionalParameters).

  Data flow: NAPI thinking_config → create_rig_agent thinking_config param →
  additional_params JSON → rig AdditionalParameters deserialization →
  CompletionRequest → wire JSON.

  The AdditionalParameters struct already has reasoning: Option<Reasoning>,
  include: Option<Vec<Include>>, parallel_tool_calls: Option<bool> fields
  that will serialize correctly if populated via additional_params.

  Reference: codex-rs builds its request in core/src/client.rs lines 500-553
  with reasoning, include, tool_choice, and parallel_tool_calls always set
  for models with supports_reasoning_summaries=true.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. create_rig_agent must use thinking_config parameter (remove underscore prefix) to populate reasoning in additional_params
  #   2. get_thinking_config() must return OpenAI Responses API reasoning format ({reasoning: {effort, summary}}) for codex and openai reasoning providers
  #   3. Responses API request must include reasoning.effort (high default), reasoning.summary (auto), include: [reasoning.encrypted_content], and tool_choice: auto
  #   4. complete_with_tools must also inject reasoning config into additional_params, not just create_rig_agent
  #   5. Default reasoning config must be applied when no thinking_config is provided (Codex models always need reasoning)
  #
  # EXAMPLES:
  #   1. Codex session with T:High sends reasoning.effort=high and reasoning.summary=auto in request, model responds with function_call items and reasoningTokens > 0
  #   2. get_thinking_config('codex', High) returns {reasoning: {effort: 'high', summary: 'auto'}}
  #   3. get_thinking_config('gpt-5.3-codex', Medium) returns {reasoning: {effort: 'medium', summary: 'auto'}}
  #   4. create_rig_agent with None thinking_config still sends default reasoning: {effort: 'high', summary: 'auto'} for Codex models
  #   5. Wire request JSON includes reasoning, include, tool_choice, and parallel_tool_calls matching codex-rs format
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have the Codex provider send reasoning configuration in Responses API requests
    So that GPT-5.3 Codex can perform multi-step agentic reasoning and tool use

  # -----------------------------------------------------------------------
  # Bug 1: get_thinking_config() has no codex/openai branch
  # -----------------------------------------------------------------------

  Scenario: get_thinking_config returns reasoning config for codex provider at High level
    Given I have the get_thinking_config function
    When I call get_thinking_config with provider "codex" and level High
    Then the returned JSON should contain a "reasoning" object
    And reasoning.effort should be "high"
    And reasoning.summary should be "auto"

  Scenario: get_thinking_config returns reasoning config for codex model name at Medium level
    Given I have the get_thinking_config function
    When I call get_thinking_config with provider "gpt-5.3-codex" and level Medium
    Then the returned JSON should contain a "reasoning" object
    And reasoning.effort should be "medium"
    And reasoning.summary should be "auto"

  Scenario: get_thinking_config returns reasoning config for codex at Low level
    Given I have the get_thinking_config function
    When I call get_thinking_config with provider "codex" and level Low
    Then the returned JSON should contain a "reasoning" object
    And reasoning.effort should be "low"
    And reasoning.summary should be "auto"

  Scenario: get_thinking_config returns empty config for codex at Off level
    Given I have the get_thinking_config function
    When I call get_thinking_config with provider "codex" and level Off
    Then the returned JSON should be an empty object

  Scenario: get_thinking_config recognizes gpt-5.1-codex as a codex model
    Given I have the get_thinking_config function
    When I call get_thinking_config with provider "gpt-5.1-codex" and level High
    Then the returned JSON should contain a "reasoning" object
    And reasoning.effort should be "high"

  # -----------------------------------------------------------------------
  # Bug 2: create_rig_agent ignores _thinking_config parameter
  # -----------------------------------------------------------------------

  Scenario: create_rig_agent uses thinking_config to populate reasoning in additional_params
    Given I have a CodexProvider instance
    And I have a thinking_config with reasoning effort "high" and summary "auto"
    When I call create_rig_agent with the thinking_config
    Then the agent additional_params should contain reasoning.effort "high"
    And the agent additional_params should contain reasoning.summary "auto"
    And the agent additional_params should contain include with "reasoning.encrypted_content"
    And the agent additional_params should contain store as false

  Scenario: create_rig_agent applies default reasoning when no thinking_config is provided
    Given I have a CodexProvider instance
    When I call create_rig_agent with None as thinking_config
    Then the agent additional_params should contain reasoning.effort "high"
    And the agent additional_params should contain reasoning.summary "auto"
    And the agent additional_params should contain include with "reasoning.encrypted_content"

  # -----------------------------------------------------------------------
  # Bug 3: complete_with_tools missing reasoning config
  # -----------------------------------------------------------------------

  Scenario: complete_with_tools includes reasoning config in additional_params
    Given I have a CodexProvider instance
    When I call complete_with_tools with messages and tools
    Then the request additional_params should contain reasoning.effort "high"
    And the request additional_params should contain reasoning.summary "auto"
    And the request additional_params should contain include with "reasoning.encrypted_content"
    And the request additional_params should contain store as false

  # -----------------------------------------------------------------------
  # Wire format matching codex-rs
  # -----------------------------------------------------------------------

  Scenario: Responses API request body matches codex-rs format
    Given I have a CodexProvider instance with reasoning configured
    When the request is serialized to JSON
    Then the JSON should contain "reasoning" with "effort" and "summary" fields
    And the JSON should contain "include" with "reasoning.encrypted_content"
    And the JSON should contain "store" as false
    And the JSON should not contain "max_output_tokens"
