@done
@completion
@providers
@codelet
@bug-fix
@streaming
@CONT-001
Feature: Multi-turn tool continuation in the rig-core streaming loop
  """
  FAULT (verified 2026-07-09): codelet/patches/rig-core/src/agent/prompt_request/streaming.rs, StreamingPromptRequest::send() async_stream loop. Exit condition line 802 `if !did_call_tool`. did_call_tool declared line 471, set true only at line 679 (ToolCall arm after call_tool), reset to false at line 564 (Text arm), line 719 (Reasoning arm), line 723 (ReasoningDelta arm). A trailing Text/Reasoning chunk AFTER a ToolCall in the same turn therefore makes the loop exit at 802 even though tools ran.
  DATA-LOSS MECHANISM: per-turn buffers tool_calls/tool_results (declared lines 543-544, pushed lines 625-626 cancel path and 676-677 normal path) are flushed to chat_history at lines 766-774 (assistant tool_use) and 777-794 (user tool_result). Line 797 then pops the LAST message (the tool_result) into current_prompt. When line 802 breaks early, that popped tool_result is dropped, leaving a dangling assistant tool_use in chat history — the 'unanswered tool result' symptom.
  FIX APPROACH (per work-unit description / failure mode F): capture per-turn tool activity, e.g. `let turn_called_tools = !tool_calls.is_empty();` evaluated BEFORE the flush at line 766, and change line 802 to exit only when the turn produced no tool calls (`if !did_call_tool && !turn_called_tools` or simply `if !turn_called_tools`), regardless of last_stop_reason. Existing max_depth guard at lines 474-478 still bounds continuation. NOTE: attached design doc spec/attachments/CONT-001/design-fmode-fix.md is 0 bytes (empty) — rules reconstructed from work-unit description + source verification; all cited line numbers re-verified against current file.
  TEST HARNESS: rig-core is NOT a codelet workspace member (codelet/Cargo.toml patches rig-core = { path = "patches/rig-core" }); its tests run via `cargo test` inside codelet/patches/rig-core. New integration test file codelet/patches/rig-core/tests/multi_turn_tool_continuation.rs uses a scripted CompletionModel mock: per-turn Vec<RawStreamingChoice<R>> queues wrapped by StreamingCompletionResponse::stream(Box::pin(...)) (same pattern as src/streaming.rs tests ~line 536), a Serialize/Deserialize StreamingResponse struct implementing GetTokenUsage::stop_reason (PROV-039), Agent built via AgentBuilder::new(model).build() with default ToolServer (unknown tool -> error-string tool result still drives the tool path at streaming.rs:676-679), driven through rig::streaming::StreamingPrompt::stream_prompt(..).multi_turn(n).
  """

  Background: User Story
    As a codelet agent-loop user
    I want to have every tool call executed during a streaming turn answered with its tool result in the next model request, even when the provider ends the turn with stop_reason stop/end_turn and trailing text or reasoning
    So that multi-step tasks continue to completion instead of stalling with unanswered tool results

  Scenario: Turn with a tool call and trailing text before stop_reason end_turn continues to a second request
    Given a scripted model whose first turn streams a tool call, then trailing text, then a final response with stop_reason "end_turn"
    And the scripted model's second turn streams only text "final answer" with stop_reason "end_turn"
    When the agent streams the prompt with multi-turn enabled
    Then the model receives a second completion request
    And the second request's chat history contains the tool result answering the first turn's tool call
    And the final response text is "final answer"

  Scenario: Turn with a tool call and a trailing reasoning delta continues to a second request
    Given a scripted model whose first turn streams a tool call, then a trailing reasoning delta, then a final response with stop_reason "end_turn"
    And the scripted model's second turn streams only text "done after reasoning" with stop_reason "end_turn"
    When the agent streams the prompt with multi-turn enabled
    Then the model receives a second completion request
    And the final response text is "done after reasoning"

  Scenario: Text-only turn exits the loop after exactly one request
    Given a scripted model whose only turn streams text "plain answer" and a final response with stop_reason "end_turn"
    When the agent streams the prompt with multi-turn enabled
    Then the model receives exactly one completion request
    And the final response text is "plain answer"

  Scenario: Tool-only turn with stop_reason tool_use still continues to a second request
    Given a scripted model whose first turn streams a tool call then a final response with stop_reason "tool_use" and no trailing chunks
    And the scripted model's second turn streams only text "after tool" with stop_reason "end_turn"
    When the agent streams the prompt with multi-turn enabled
    Then the model receives a second completion request
    And the final response text is "after tool"
