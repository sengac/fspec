@done
@hooks
@RPC-090
@rust
@agent-loop
@lifecycle-hooks
@source-shape
@rpc-090
Feature: Agent loop: lifecycle hooks (session_start / user_prompt_submit / post_tool_use / session_end)

  """
  Regression-shape feature: source-string assertions over
  agent_loop.rs + background_output.rs only — no LifecycleHooks
  instance is constructed at test time.

  Behavioural feature `agent-loop-lifecycle-hooks.feature` remains
  `@deferred` without an owner; this card scopes itself to a new
  dedicated source-shape feature (same pattern as RPC-089/RPC-077).

  Tests target `codelet/agent-loop/src/agent_loop.rs` +
  `codelet/agent-loop/src/background_output.rs` via Cargo workspace
  path traversal — sub-millisecond assertions, no async runtime.
  """

  Background: User Story
    As a fspec contributor
    I want to have HOOK-013 lifecycle-hook invocation points pinned as regression-shape invariants
    So that the RPC-072 stub state (no hooks invoked) cannot silently re-emerge

  Scenario: agent_loop.rs imports all four loop-side hook runners by name
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "run_session_end"
    And the source must contain "run_session_start"
    And the source must contain "run_user_prompt"
    And the source must contain "HookMessageLevel"
    And the source must contain "use codelet_core::lifecycle_hooks::"

  Scenario: run_session_start fires with the literal phase string "startup"
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "run_session_start(hooks, &ctx, \"startup\").await"

  Scenario: run_session_end fires with the literal phase string "exit" before break
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "run_session_end(hooks, &ctx, \"exit\").await"
    And the source must contain "break;"

  Scenario: run_user_prompt is guarded by !hooks.user_prompt_submit.is_empty()
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "!hooks.user_prompt_submit.is_empty()"
    And the source must contain "run_user_prompt(hooks, &ctx, input).await"
    And the offset of "!hooks.user_prompt_submit.is_empty()" must be less than the offset of "run_user_prompt(hooks, &ctx, input).await"

  Scenario: user_prompt_submit block path emits ordered Idle / done / continue tokens
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "!outcome.allow_prompt"
    And the source must contain "set_status(SessionStatus::Idle)"
    And the source must contain "StreamChunk::done()"
    And the source must contain "continue;"
    And the offset of "set_status(SessionStatus::Idle)" must be less than the offset of "StreamChunk::done()" appearing after "!outcome.allow_prompt"

  Scenario: session_start invocation precedes the main loop opening
    Given I read the source of codelet/agent-loop/src/agent_loop.rs
    When I scan the file as a string
    Then the source must contain "run_session_start(hooks, &ctx, \"startup\").await"
    And the source must contain "loop {"
    And the offset of "run_session_start(hooks, &ctx, \"startup\").await" must be less than the offset of the first "loop {"

  Scenario: background_output.rs imports run_post_tool from codelet_core::lifecycle_hooks
    Given I read the source of codelet/agent-loop/src/background_output.rs
    When I scan the file as a string
    Then the source must contain "use codelet_core::lifecycle_hooks::{run_post_tool, HookMessageLevel};"

  Scenario: run_post_tool is invoked inside a tokio::spawn closure with five arguments
    Given I read the source of codelet/agent-loop/src/background_output.rs
    When I scan the file as a string
    Then the source must contain "tokio::spawn(async move"
    And the source must contain "run_post_tool("
    And the source must contain each of the five canonical run_post_tool arguments
    And the offset of "tokio::spawn(async move" must be less than the offset of "run_post_tool("

  Scenario: post_tool_use uses last_tool_call captured during StreamEvent::ToolCall
    Given I read the source of codelet/agent-loop/src/background_output.rs
    When I scan the file as a string
    Then the source must contain "last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>"
    And the source must contain "self.last_tool_call"
    And the source must contain "!hooks.post_tool_use.is_empty()"
