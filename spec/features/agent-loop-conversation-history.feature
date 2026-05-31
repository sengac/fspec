@wip
@deferred
@session-management
@RPC-081
@rust
@agent-loop
@rpc
@history
Feature: Agent loop round-trips conversation history between turns
  """
  RPC-081 (child of RPC-072 family). Every turn must read session.inner.messages
  before the LLM call so the second prompt sees the first turn's
  user+assistant messages — codelet/napi/src/agent_loop.rs parity.

  Originally scenario "Conversation history round-trips between turns"
  from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want each follow-up prompt to remember what came before
    So that conversational context survives across turns just like the TS Ink frontend

  Scenario: Conversation history round-trips between turns
    Given a Work Agent session backed by a deterministic stub provider with chat-memory behaviour
    And the user sends "remember 42" and receives an assistant reply
    When the user sends "what number did I just give you" in the same session
    Then the stub provider's recorded prompt for the second turn contains both prior user and assistant messages
    And the assistant reply contains "42"
