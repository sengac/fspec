@done
@BUG-440
@providers
@thinking-detection
@rust
Feature: OpenAI-compatible providers send the selected thinking level
  """
  The OpenAI Chat Completions provider (and so every local vLLM / SGLang /
  Strata profile), Z.AI and GitHub Copilot ignored their thinking_config
  argument, and get_thinking_config returned {} for their model ids, so the
  selected level was never sent although the TUI showed it as active
  (MODEL-009 research §1.3, §5.3).
  get_thinking_config now has provider branches: "openai" and
  "github-copilot" give {"reasoning_effort": "low|medium|high"} (Off gives
  {}: no reasoning parameter), "zai" gives {"thinking": {"type":
  "enabled"}} or {"type": "disabled"} for Off. resolve_turn_thinking routes
  these providers by the dispatched provider name before the model-id
  routing (a Claude id served by Copilot is a Copilot request), including
  Off. The providers merge the config into the request's additional
  parameters (codelet_providers::merge_additional_params), keeping the local
  anti-ramble sampling parameters.
  MODEL-010 (thinking-levels-reach-each-provider-through-a-wire-table.feature)
  replaced get_thinking_config with the provider wire table: Off is now
  reasoning_effort none, a local profile also gets chat_template_kwargs
  thinking / enable_thinking, and the Z.AI enabled mapping is gone (Z.AI
  has only the toggle's off form; product owner, 2026-11-04).
  MODEL-016 (local-inference-server-thinking-levels.feature) replaced the
  fixed local-profile template toggles: a local profile model gets the levels
  it declares in its profile, in its engine's form, and a model that declares
  none has no reasoning control.
  Tests: rust/agent-loop/tests/bug440_openai_compatible_thinking_level.rs.
  """

  Background: User Story
    As a user of an OpenAI-compatible provider (OpenAI, a local vLLM/SGLang profile, Z.AI or Copilot)
    I want to have the thinking level I select sent with each request
    So that the level shown as active is the level the model is asked for

  Scenario: OpenAI and Copilot levels become reasoning_effort
    Given the providers "openai" and "github-copilot"
    When the Low, Medium, High and Off thinking levels are configured
    Then Low, Medium and High give reasoning_effort low, medium and high and Off gives reasoning_effort none

  Scenario: A local OpenAI-compatible server receives reasoning_effort with its sampling parameters
    Given an OpenAI provider pointed at a local vLLM server whose model declares efforts "low, medium, high" (MODEL-016)
    When a request is sent with the High thinking level
    Then the request body carries reasoning_effort "high" and the anti-ramble top_p, and no template toggle the model does not declare
