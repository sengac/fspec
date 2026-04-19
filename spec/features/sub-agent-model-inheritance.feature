@agent-manager
@codelet
@BUG-132
Feature: DeepSearch and AgentManager handlers use stale model after mid-session model switch

  """
  Extract handler re-registration logic into a shared helper function to avoid duplicating the closure-building code between initial registration and model-change re-registration
  session_set_model and session_set_model_profile are NAPI-exported async functions called from TypeScript TUI — changes must preserve their signatures and return types
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. DeepSearch and AgentManager handlers must re-register when session_set_model or session_set_model_profile changes the active model
  #   2. DeepSearch handler must check facade_override() before current_provider_name(), matching the agent_loop pattern at line 4744-4747
  #   3. The re-registration must update all four captured values: provider_name, model_id, context_window, max_output_tokens
  #   4. AgentManager re-registration must use selected_model_string() (registry format) not current_provider_name()/current_model_id() (internal format), per AMGR-013
  #   5. All existing Rust tests (cargo test) must continue to pass after the fix
  #
  # EXAMPLES:
  #   1. User creates session with claude-sonnet-4-20250514, switches to gemini-2.5-pro, invokes DeepSearch → sub-agent should use gemini-2.5-pro (not claude-sonnet)
  #   2. User creates session with claude-sonnet-4-20250514, switches to gemini-2.5-pro, spawns AgentManager subordinate → subordinate should use gemini-2.5-pro (not claude-sonnet)
  #   3. User with MODEL-004 custom model (facade_override routing openai→claude) invokes DeepSearch → sub-agent should dispatch to claude backend, not openai
  #   4. User never switches model mid-session → behavior is unchanged from before the fix (no regression)
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have DeepSearch and AgentManager sub-agents use my current model after I switch models mid-session
    So that sub-agents behave consistently with my main session

  Scenario: DeepSearch uses updated model after mid-session model switch
    Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    And the DeepSearch handler was registered at session creation
    When the user switches the model to "google/gemini-2.5-pro" via session_set_model
    And the user invokes DeepSearch
    Then the DeepSearch sub-agent should use provider "gemini" and model "gemini-2.5-pro"

  Scenario: AgentManager uses updated model after mid-session model switch
    Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    And the AgentManager handler was registered at session creation
    When the user switches the model to "google/gemini-2.5-pro" via session_set_model
    And the user spawns a subordinate via AgentManager
    Then the subordinate should be created with the updated model "gemini-2.5-pro"

  Scenario: DeepSearch respects facade_override for custom models
    Given a session was created with a MODEL-004 custom model registered under "openai" with facade_override "claude"
    When the user invokes DeepSearch
    Then the DeepSearch sub-agent should use provider "claude" not "openai"

  Scenario: Handler re-registration updates all four captured values
    Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    When the user switches the model to "google/gemini-2.5-pro" via session_set_model with context_window 1048576 and max_output_tokens 65536
    Then the DeepSearch handler should capture provider "gemini", model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536
    And the AgentManager handler should capture model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536

  Scenario: No regression when model is never changed
    Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    And no model switch occurs during the session
    When the user invokes DeepSearch
    Then the DeepSearch sub-agent should use provider "claude" and model "claude-sonnet-4-20250514"

  Scenario: session_set_model_profile also triggers handler re-registration
    Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    When the user switches the model via session_set_model_profile to provider "openai" model "gpt-4o"
    And the user invokes DeepSearch
    Then the DeepSearch sub-agent should use provider "openai" and model "gpt-4o"
