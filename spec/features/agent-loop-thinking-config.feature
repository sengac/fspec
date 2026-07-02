@done
@session-management
@RPC-085
@rust
@agent-loop
@rpc
@thinking
Feature: Agent loop threads /thinking high into the provider request
  """
  RPC-085 (child of RPC-072 family). Thinking config must be computed
  per turn from session.inner.session_thinking_level +
  PromptInput.thinking_config + detected_level (BRIDGE-006 / PROV-005 /
  PROV-041) and threaded through create_rig_agent into provider
  additional_params. The computation has three priority branches
  (PROV-005 fix):

  1. Adaptive thinking models ALWAYS use model-aware config and
  override any TS-passed `PromptInput.thinking_config` (prevents
  Opus 4.6 `max_tokens` rejection).
  2. Non-adaptive models honour `PromptInput.thinking_config` verbatim.
  3. Otherwise, unified detection from message text + session base
  level (with `has_disable_keywords` force-off).

  The resulting `Option<serde_json::Value>` must reach all three
  dispatch paths: the `run_with_provider!` macro (claude / gemini /
  zai / codex / copilot), the inlined `openai` arm, and the
  custom-provider fallthrough.
  """

  Background: User Story
    As a fspec maintainer
    I want the agent loop to thread an effective thinking_config through create_rig_agent for every dispatch arm
    So that /thinking high, adaptive models, and PromptInput.thinking_config all influence the on-wire provider request without drifting between arms

  Scenario: InputWithImages carries thinking_config alongside text and images
    Given the dispatch helper struct in codelet/agent-loop/src/dispatch.rs
    When the structural source is inspected
    Then InputWithImages declares a thinking_config field of type Option<String>
    And the field documents that it is a per-turn override superimposed on session_thinking_level

  Scenario: run_with_provider! macro forwards thinking config to create_rig_agent
    Given the run_with_provider! macro_rules! body in codelet/agent-loop/src/dispatch.rs
    When the macro body is parsed
    Then the macro accepts a $thinking metavariable as its 7th positional argument
    And the macro invokes provider.create_rig_agent with role_preamble.as_deref() and $thinking.clone() as the 2nd and 3rd positional arguments

  Scenario: agent_loop body computes thinking_config_value per turn
    Given the agent loop body in codelet/agent-loop/src/agent_loop.rs
    When the source is scanned
    Then a thinking_config_value binding of type Option<serde_json::Value> is computed once per turn
    And the computation references compute_effective_thinking_level, is_adaptive_thinking_model, and get_thinking_config
    And the computation honours the PROV-005 priority order (adaptive first, then TS-passed config, then unified detection)

  Scenario: All run_with_provider! call sites pass thinking_config_value
    Given the agent loop body in codelet/agent-loop/src/agent_loop.rs
    When every invocation of the run_with_provider! macro is enumerated
    Then each invocation passes thinking_config_value as its 7th positional argument
    And the enumerated providers cover claude, gemini, zai, codex, and copilot

  Scenario: OpenAI inlined arm passes thinking_config_value to create_rig_agent
    Given the inlined "openai" => { ... } match arm in codelet/agent-loop/src/agent_loop.rs
    When the arm body is parsed
    Then provider.create_rig_agent is invoked with session.id, role_preamble.as_deref(), and thinking_config_value.clone() as the 1st, 2nd, and 3rd positional arguments

  Scenario: Custom-provider fallthrough passes thinking_config_value to create_rig_agent
    Given the `_ =>` custom-provider fallthrough match arm in codelet/agent-loop/src/agent_loop.rs
    When the arm body is parsed
    Then codelet_providers::custom::CustomProvider::create_rig_agent is invoked
    And the final three positional arguments are session.id, role_preamble.as_deref(), and thinking_config_value.clone()

  Scenario: Provider create_rig_agent signature compiles for all 6 instance-based built-in providers
    Given the create_rig_agent method on each of Claude, OpenAI, Gemini, ZAI, Codex, and Copilot
    When a no-op closure pins the signature (uuid::Uuid, Option<&str>, Option<serde_json::Value>) -> RigAgentHandle
    Then the closure compiles for every provider type without coercion

  Scenario: CustomProvider create_rig_agent signature accepts thinking_config as its last argument
    Given the free function codelet_providers::custom::CustomProvider::create_rig_agent
    When a no-op closure pins the signature (&Path, &str, &str, uuid::Uuid, Option<&str>, Option<serde_json::Value>)
    Then the closure compiles without coercion
    And thinking_config is the 6th positional argument
