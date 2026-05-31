@wip
@deferred
@session-management
@RPC-086
@rust
@agent-loop
@rpc
@tokens
Feature: Agent loop emits TokenUpdate and ContextFillUpdate from BackgroundOutput
  """
  RPC-086 (child of RPC-072 family). Token tracking must call
  session.update_tokens(input, output) + session.update_reasoning_tokens(reasoning)
  from BackgroundOutput on every StreamEvent::Tokens, and emit
  StreamChunk::TokenUpdate + StreamChunk::ContextFillUpdate so the
  TUI tokens widget moves.

  Originally scenario "TokenUpdate chunks update the session counters
  and TUI header widget" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want the TUI tokens widget to update as the LLM consumes context
    So that I can see input/output token cost in real time

  Scenario: TokenUpdate chunks update the session counters and TUI header widget
    Given a Work Agent session backed by a stub provider that returns Usage{input: 100, output: 50}
    When the user sends "hi"
    Then session.get_tokens() returns input=100 and output=50
    And the TUI session header widget displays "tokens: 100↓ 50↑"
    And a StreamChunk::ContextFillUpdate arrived with a non-zero fill_percentage
