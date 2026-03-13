@BUG-102
Feature: DeepSearch tool fails with 'Model is required' configuration error

  """
  The deep_search_handler.rs comment says 'v1: Claude-only' but the fix should be provider-agnostic using with_provider_and_model() which works for any provider
  Changes needed: (1) deep_search_handler::execute_deep_search() takes provider_name + model_id params, (2) build_and_run_agent() uses with_provider_and_model() + dynamic provider getter, (3) session_manager handler closure captures provider/model from parent session inner lock, (4) DeepSearchHandler type signature adds provider_name + model_id params
  """

  Background: User Story
    As a AI agent
    I want to use the DeepSearch tool to explore code and session history
    So that I can answer complex questions requiring multi-file exploration

  Scenario: Sub-agent inherits Claude provider and model from parent session
    Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
    When the DeepSearch handler is registered for the session
    Then the handler closure captures the provider name "claude"
    And the handler closure captures the model id "claude-sonnet-4-20250514"
    And the sub-agent creates a ProviderManager with provider "claude" and model "claude-sonnet-4-20250514"

  Scenario: Sub-agent inherits OpenAI provider and model from parent session
    Given a parent session with provider "openai" and model "gpt-4o"
    When the DeepSearch handler is registered for the session
    Then the handler closure captures the provider name "openai"
    And the handler closure captures the model id "gpt-4o"
    And the sub-agent creates a ProviderManager with provider "openai" and model "gpt-4o"

  Scenario: Sub-agent inherits Codex provider and model from parent session
    Given a parent session with provider "codex" and model "gpt-5.1-codex"
    When the DeepSearch handler is registered for the session
    Then the handler closure captures the provider name "codex"
    And the handler closure captures the model id "gpt-5.1-codex"
    And the sub-agent creates a ProviderManager with provider "codex" and model "gpt-5.1-codex"

  Scenario: Sub-agent inherits Z.AI provider and model from parent session
    Given a parent session with provider "zai" and model "glm-4.7"
    When the DeepSearch handler is registered for the session
    Then the handler closure captures the provider name "zai"
    And the handler closure captures the model id "glm-4.7"
    And the sub-agent creates a ProviderManager with provider "zai" and model "glm-4.7"

  Scenario: Sub-agent uses with_provider_and_model instead of with_model_support
    Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
    When the DeepSearch sub-agent builds a ProviderManager
    Then the ProviderManager is created via with_provider_and_model
    And select_model is not called
    And the selected_model_id returns "claude-sonnet-4-20250514"
