@done
@provider-abstraction
@codex
@providers
@regression
@critical
@BUG-208
Feature: Codex live agent path with all production tools

  """
  Architecture notes:
  - Live regression guard for BUG-208: exercises the FULL production agent path
    (CodexProvider::create_rig_agent with all Codex-native tool facades +
    stream_prompt(...).multi_turn()), exactly what RigAgent::prompt_streaming_with_history
    does in the TUI.
  - Uses real Codex OAuth credentials from ~/.codex/auth.json; the test skips
    (returns early) when no credentials exist on the machine.
  - The BUG-208 fix lives in the patched rig-core: schema sanitizer + strict:false
    in ResponsesToolDefinition::from (providers/openai/responses_api/mod.rs), and
    SSE error-event surfacing in responses_api/streaming.rs.
  """

  Background: User Story
    As an agent user
    I want to converse with Codex-backend models (gpt-6-astra) through the full agent loop with all production tools registered
    So that the conversation produces a real response instead of silently terminating with no output

  Scenario: Live gpt-6-astra agent conversation with all production tools produces a response
    Given real Codex OAuth credentials and the gpt-6-astra model
    And the full production agent with all Codex-native tools registered (including AstGrepRefactor)
    When the agent is prompted "Use the shell_command tool to run: echo pong. Then tell me the output." with multi-turn enabled
    Then the agent stream produces tool calls and/or streamed text
    And the final response is non-empty
    And no invalid_function_parameters error occurs
