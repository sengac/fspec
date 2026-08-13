@done
@session-management
@RPC-084
@rust
@agent-loop
@rpc
@streaming
Feature: Agent loop streaming via run_agent_stream_with_images + all 19+ StreamChunk variants
  """
  RPC-084 (child of RPC-072 family). Streaming must go through
  codelet_cli::interactive::run_agent_stream_with_images — the same rig
  multi-turn streaming engine the NAPI loop uses. Non-streaming
  complete_with_tools is forbidden as the primary path.

  Parity is enforced structurally across the three dispatch arms
  (run_with_provider! macro body in dispatch.rs, the OpenAI inlined arm
  in agent_loop.rs, and the custom-provider fallthrough arm in
  agent_loop.rs) plus a variant census of codelet_rpc_types::StreamChunk
  and the BackgroundOutput translator that maps rig StreamEvent variants
  to StreamChunk variants.

  Gap analysis sections §5, §13 in
  spec/attachments/RPC-084/agent-loop-parity-gap.md.
  """

  Background: User Story
    As a fspec user
    I want to see assistant replies stream in as multiple deltas
    So that I get rig multi-turn streaming and the full StreamChunk variant set the NAPI loop emits

  Scenario: run_with_provider! macro body in dispatch.rs uses run_agent_stream_with_images as the streaming dispatch path
    Given the source file rust/agent-loop/src/dispatch.rs
    When I locate the run_with_provider! macro body
    Then the body contains exactly one call to `codelet_cli::interactive::run_agent_stream_with_images`
    And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    And the call passes the 9 positional arguments in the canonical order: agent, $input, $images, $inner, $session.is_interrupted.clone(), $session.compaction_in_progress.clone(), $session.interrupt_notify.clone(), $output, $session.id

  Scenario: OpenAI inlined arm in agent_loop.rs mirrors the macro body and calls run_agent_stream_with_images
    Given the source file rust/agent-loop/src/agent_loop.rs
    When I locate the "openai" match arm body
    Then the arm contains exactly one direct call to `codelet_cli::interactive::run_agent_stream_with_images`
    And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    And the call is positioned between line 950 and line 1050

  Scenario: Custom-provider fallthrough arm wraps the rig agent and calls run_agent_stream_with_images
    Given the source file rust/agent-loop/src/agent_loop.rs
    When I locate the `_ =>` fallthrough match arm body
    Then the arm contains exactly one call to `codelet_cli::interactive::run_agent_stream_with_images`
    And the call appears after `codelet_core::RigAgent::with_default_depth(agent)`
    And the call is positioned between line 1000 and line 1260

  Scenario: Non-streaming complete_with_tools is forbidden as the primary dispatch path in the agent loop body
    Given the source file rust/agent-loop/src/agent_loop.rs
    When I scan non-comment, non-test lines of the file
    Then there is no `.complete_with_tools(` invocation in the agent loop body
    And every line matching `.complete_with_tools(` belongs to either a `//` comment or a `#[cfg(test)]` test module

  Scenario: codelet_rpc_types::StreamChunk exposes at least nineteen variants matching the NAPI emission set
    Given the source file rust/rpc-types/src/lib.rs
    When I locate the `pub enum StreamChunk` declaration
    Then the enum declares 19 or more variants
    And the variant set includes Text, Thinking, ToolCall, ToolResult, ToolProgress, SessionStateChange, UserNotification, Interrupted, TokenUpdate, ContextFillUpdate, Done, Error, UserInput, IncomingMessage, SupervisorPendingInjection, CompactionComplete, FspecCommandRequest, FspecCommandResult, WorkUnitsUpdate

  Scenario: BackgroundOutput translates the eleven canonical rig StreamEvent variants into StreamChunk variants
    Given the source file rust/agent-loop/src/background_output.rs
    When I scan the handle_stream_event match arms
    Then each of the following StreamChunk constructors appears at least once: StreamChunk::text, StreamChunk::thinking, StreamChunk::tool_call, StreamChunk::tool_result, StreamChunk::tool_progress, StreamChunk::user_notification, StreamChunk::token_update, StreamChunk::context_fill_update, StreamChunk::error, StreamChunk::interrupted, StreamChunk::done

  Scenario: run_agent_stream_with_images public signature accepts the canonical 9 positional arguments
    Given the codelet_cli::interactive module exposes run_agent_stream_with_images
    When I take a closure reference to the function with the canonical 9-argument signature
    Then the closure compiles
    And the function is re-exported from codelet_cli::interactive so the agent loop dispatch arms can call it directly
