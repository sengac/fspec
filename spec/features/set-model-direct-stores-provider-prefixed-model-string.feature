@agent-manager
@codelet
@BUG-136
Feature: AgentManager spawn fails for custom models with slashes in model_id
  """
  Root cause in codelet/providers/src/manager.rs: set_model_direct (line ~476) stores only model_id, whereas select_model (line ~431) stores the full provider/model composite. selected_model_string() returns this stored value verbatim, so AgentManager's handle_spawn passes an incomplete string to create_session_with_id. Fix stays Rust-only and needs the companion update in selected_model_id (line ~495) to continue returning the bare API id.
  Existing BUG-132 regression test at codelet/napi/src/session_manager.rs:8308-8316 asserts selected_model_string() == 'claude-opus-4-6' — this test encodes the buggy behaviour and MUST be updated to expect 'anthropic/claude-opus-4-6'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. set_model_direct MUST store the full 'provider_id/model_id' composite so selected_model_string() always returns a registry-parseable string, matching select_model()'s behaviour
  #   2. selected_model_id() MUST continue to return just the bare API model id (no provider prefix) so provider-specific get_*() constructors get the id they expect — stripping the prefix when registry lookup fails and the stored string still starts with provider_id/
  #   3. The stripping logic MUST only remove the FIRST slash segment (the provider prefix) so model ids that legitimately contain slashes (e.g. 'accounts/fireworks/models/kimi-k2-06-instruct') are preserved intact
  #
  # EXAMPLES:
  #   1. AgentManager spawn with a custom Rhai provider whose model id is 'accounts/fireworks/models/kimi-k2-06-instruct' succeeds — the subordinate is created with the same provider/model and no 'Unknown provider' error is raised
  #   2. After set_model_direct('my-llm', 'llama-3.1-70b', ...), selected_model_string() returns 'my-llm/llama-3.1-70b' (not 'llama-3.1-70b')
  #   3. After set_model_direct('my-llm', 'accounts/fireworks/models/kimi-k2-06-instruct', ...), selected_model_id() returns 'accounts/fireworks/models/kimi-k2-06-instruct' (unchanged — only the 'my-llm/' prefix is stripped)
  #   4. Existing Codex models (set_model_direct('codex', 'gpt-5-codex', ...)) continue to behave correctly — selected_model_string() = 'codex/gpt-5-codex', selected_model_id() = 'gpt-5-codex'
  #   5. Existing Anthropic/Google/OpenAI registry selections via select_model('anthropic/claude-opus-4-6') continue to work — selected_model_string() = 'anthropic/claude-opus-4-6', selected_model_id() = resolved API id from registry (unchanged from today)
  #
  # ========================================
  Background: User Story
    As a developer using a custom model whose API id contains slashes
    I want to spawn subordinate sessions via AgentManager
    So that parallel work and research are not broken for Fireworks/OpenRouter/aggregator-style providers

  Scenario: Custom model with slashes in model_id can spawn AgentManager subordinate
    Given a provider manager has been configured via set_model_direct with provider "openai" and model id "accounts/fireworks/models/kimi-k2-06-instruct"
    When AgentManager registers its spawn handler using selected_model_string
    Then the captured model string is "openai/accounts/fireworks/models/kimi-k2-06-instruct"
    And passing that captured model string to create_session_with_id resolves the provider as "openai"
    And no "Unknown provider: 'accounts'" error is raised

  Scenario: set_model_direct stores the full provider-prefixed model string
    Given a fresh provider manager
    When set_model_direct is called with provider "openai" and model id "llama-3.1-70b"
    Then selected_model_string returns "openai/llama-3.1-70b"
    And selected_model_id returns "llama-3.1-70b"

  Scenario: selected_model_id strips only the first slash segment when the model id contains slashes
    Given a fresh provider manager
    When set_model_direct is called with provider "openai" and model id "accounts/fireworks/models/kimi-k2-06-instruct"
    Then selected_model_string returns "openai/accounts/fireworks/models/kimi-k2-06-instruct"
    And selected_model_id returns "accounts/fireworks/models/kimi-k2-06-instruct"

  Scenario: Codex models continue to work after the fix
    Given a fresh provider manager
    When set_model_direct is called with provider "codex" and model id "gpt-5-codex"
    Then selected_model_string returns "codex/gpt-5-codex"
    And selected_model_id returns "gpt-5-codex"

  Scenario: with_provider_and_model emits a registry-format composite
    Given valid "anthropic" credentials
    When with_provider_and_model is called with provider "claude" and model id "claude-opus-4-6"
    Then selected_model_string returns "claude/claude-opus-4-6"
    And selected_model_id returns "claude-opus-4-6"
