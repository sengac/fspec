@done
@RPC-081 @rpc @rust @agent-loop @session-management @history @wip
Feature: Agent loop body threads &mut session.inner.messages into the rig streaming engine every turn

  """
  RPC-081 (child of RPC-072 family). Source-shape regression guarding the
  conversation-history threading. Before RPC-072 the stub at
  codelet/agent-loop/src/agent_loop.rs:78-81 rebuilt `vec![Message { role:
  MessageRole::User, ... }]` every turn — the LLM had no chat memory.

  After the fix, the agent loop body MUST thread `&mut inner_session` into
  `codelet_cli::interactive::run_agent_stream_with_images` (direct call site
  for the openai/custom-provider arms) AND as the first argument to every
  `run_with_provider!(...)` macro invocation. Both pathways forward
  `&mut session.inner.messages` into the rig streaming engine, which is what
  makes conversation history round-trip across turns (canonical site
  codelet/cli/src/interactive/stream_loop.rs:461-471).
  """

  Background: User Story
    As a fspec user
    I want to have each follow-up prompt remember the prior turns in the same session
    So that conversational context survives across turns just like the TypeScript Ink frontend

  Scenario: Source-shape — agent_loop body never constructs a single-element user-only history vec
    Given the source of codelet/agent-loop/src/agent_loop.rs is read into memory with Rust comments stripped
    When the body is scanned for the literal pattern "vec![Message { role: MessageRole::User"
    Then no match is found
    And no match is found for "vec![Message { role: rig::message::MessageRole::User"

  Scenario: Source-shape — agent_loop body threads &mut inner_session into run_agent_stream_with_images
    Given the source of codelet/agent-loop/src/agent_loop.rs is read into memory with Rust comments stripped
    When the body is scanned for occurrences of "&mut inner_session"
    Then at least one occurrence appears as the fourth positional argument to "codelet_cli::interactive::run_agent_stream_with_images"
    And at least one occurrence appears as the first argument to a "run_with_provider!" macro invocation
