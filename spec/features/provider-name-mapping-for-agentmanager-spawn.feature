@done
@agent-manager
@AMGR-013
Feature: Provider name mapping for AgentManager spawn

  """
  Bug fix: agent_manager_handler.rs handle_spawn previously reconstructed the model
  string from separate provider_id + model_id, using internal ProviderType names
  (claude, gemini) that don't match the registry names create_session_with_id expects.
  Fix: pass through the full model string from ProviderManager::selected_model_string()
  which already has the correct registry format (e.g. 'anthropic/claude-opus-4-6').
  """

  Background:
    Given an active session using a specific provider and model

  Scenario: Spawn subordinate with Anthropic provider passes correct model string
    Given the spawner's selected_model_string returns "anthropic/claude-opus-4-6"
    When the agent calls AgentManager spawn action
    Then the model string "anthropic/claude-opus-4-6" is passed to create_session_with_id
    And the subordinate session should be created successfully

  Scenario: Spawn subordinate with Google provider passes correct model string
    Given the spawner's selected_model_string returns "google/gemini-2.5-pro"
    When the agent calls AgentManager spawn action
    Then the model string "google/gemini-2.5-pro" is passed to create_session_with_id
    And the subordinate session should be created successfully

  Scenario: Spawn subordinate with OpenAI provider passes through unchanged
    Given the spawner's selected_model_string returns "openai/gpt-4o"
    When the agent calls AgentManager spawn action
    Then the model string "openai/gpt-4o" is passed to create_session_with_id
    And the subordinate session should be created successfully
