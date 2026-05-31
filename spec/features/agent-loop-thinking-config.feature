@wip
@deferred
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
  additional_params.

  Originally scenario "/thinking high threads the thinking_config into
  the provider request" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want my /thinking high command to actually reach the LLM as a thinking_config
    So that the model engages its reasoning mode end-to-end

  Scenario: /thinking high threads the thinking_config into the provider request
    Given a Work Agent session backed by a stub provider that records its additional_params
    When the user runs "/thinking high" then sends "solve this"
    Then the stub provider observed a non-None thinking_config in additional_params
    And the thinking_config encodes either {"type": "enabled", "budget_tokens": <N>} or the adaptive equivalent for the active model
