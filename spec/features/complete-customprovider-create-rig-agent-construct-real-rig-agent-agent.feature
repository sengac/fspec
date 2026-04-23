@done
@providers
@provider-abstraction
@PROV-092
Feature: Complete CustomProvider::create_rig_agent — construct real rig::agent::Agent
  """
  Replaces the opaque CustomRigAgent shim with a fully wired rig::agent::Agent<RhaiCustomProviderModel>. Uses rig builder pattern mirroring claude.rs/codex/mod.rs/copilot/rig_agent.rs. RhaiCustomProviderModel implements rig::completion::CompletionModel and bridges rig CompletionRequest to RhaiCustomProvider::invoke_build_url/headers/request/parse_response, and rig stream() to open_stream() converting StreamChunk to rig RawStreamingChoice. RhaiToolWrapper implements rig::tool::Tool (with dummy const NAME, override name()) and dispatches via apply_map_tool_params + default_to_internal to internal tool impls. thinking_config flows from agent builder additional_params through to Rhai build_request. The session_manager.rs agent_loop dispatch matches on the custom provider name and routes through CustomProvider::create_rig_agent.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CustomProvider::create_rig_agent returns a rig::agent::Agent<RhaiCustomProviderModel> instead of an opaque CustomRigAgent shim
  #   2. RhaiCustomProviderModel implements rig::completion::CompletionModel — completion() bridges rig CompletionRequest to RhaiCustomProvider::invoke_build_url/headers/request and parse_response
  #   3. RhaiCustomProviderModel::stream() opens an SSE stream via open_stream and converts custom::stream::StreamChunk into rig::streaming::RawStreamingChoice — TextDelta→Message, ReasoningDelta→ReasoningDelta, ToolCallComplete→ToolCall, StopReason→FinalResponse with stop_reason
  #   4. thinking_config (PROV-090) supplied to create_rig_agent is forwarded into RhaiCustomProviderModel and reaches Rhai build_request via the request.thinking_config field
  #   5. Each RhaiToolFacadeAdapter is wrapped in a RhaiToolWrapper that implements rig::tool::Tool — name() returns the dynamic Rhai-defined tool name and call() dispatches via apply_map_tool_params + default_to_internal to the correct internal tool implementation
  #   6. The system_prompt facade (RhaiSystemPromptFacade) composed via transform_preamble is passed as the agent preamble; identity_prefix injection from the script is applied
  #
  # EXAMPLES:
  #   1. User runs /login my-script provider, sends a prompt; CustomProvider::create_rig_agent constructs an Agent that calls Rhai build_request, posts to the URL, and parse_response yields assistant text streamed back to the TUI
  #   2. Script defines a tool 'read_file' with maps_to file:read; when the LLM calls read_file, the tool result returned to the model contains the file contents read by the internal ReadTool
  #   3. When the model emits a streaming reasoning_delta SSE event the agent loop sees a StreamedAssistantContent::ReasoningDelta chunk and renders it in the thinking pane
  #   4. thinking_config supplied to create_rig_agent (e.g. {budget_tokens:8000}) is observable inside Rhai build_request as request.thinking_config and round-trips into the outbound HTTP body unchanged
  #   5. User selects current_provider='my-script' in session_set_model_profile; agent_loop dispatches via the new custom arm and the entire conversation flows through Rhai with no built-in provider involvement
  #
  # ========================================
  Background: User Story
    As a fspec agent loop
    I want to dispatch a custom-provider session through CustomProvider::create_rig_agent which returns a real rig::agent::Agent backed by a RhaiCustomProviderModel CompletionModel and RhaiToolWrapper instances
    So that Rhai shadow scripts actually drive the LLM conversation end-to-end

  Scenario: create_rig_agent returns a real rig::agent::Agent specialised over RhaiCustomProviderModel
    Given a custom provider config "my-script" exists with a Rhai script defining build_url, build_headers, build_request, parse_response
    When the agent loop calls CustomProvider::create_rig_agent with name "my-script", model_alias "default", and a session_id
    Then the call returns Ok with a value whose static type is rig::agent::Agent<RhaiCustomProviderModel>
    And the returned agent has been built via rig::agent::AgentBuilder::build()

  Scenario: RhaiCustomProviderModel::completion bridges rig CompletionRequest through the Rhai contract
    Given a RhaiCustomProviderModel constructed from a "my-script" provider whose build_request returns the messages array unchanged
    And a rig CompletionRequest with chat_history containing a single user message "hello" and tools=[]
    When rig calls model.completion(request)
    Then RhaiCustomProvider::invoke_build_url is invoked exactly once
    And RhaiCustomProvider::invoke_build_headers is invoked exactly once
    And RhaiCustomProvider::invoke_build_request is invoked with the converted message slice and the request.thinking_config field bridged from request.additional_params
    And the returned rig CompletionResponse choice contains an AssistantContent::Text matching the script's parse_response text

  Scenario: RhaiCustomProviderModel::stream converts StreamChunk into rig RawStreamingChoice
    Given a RhaiCustomProviderModel whose script's parse_stream_chunk emits a text_delta then a reasoning_delta then a tool_call_delta then a stop end_turn
    When rig calls model.stream(request) and the stream is polled to completion
    Then a RawStreamingChoice::Message is yielded for the text_delta
    And a RawStreamingChoice::ReasoningDelta is yielded for the reasoning_delta
    And a RawStreamingChoice::ToolCall is yielded after the tool-call accumulator flushes
    And a RawStreamingChoice::FinalResponse is yielded carrying the EndTurn stop_reason

  Scenario: thinking_config supplied to create_rig_agent reaches Rhai build_request via request.thinking_config
    Given a custom provider script whose build_request echoes its input back as the JSON body
    When CustomProvider::create_rig_agent is called with thinking_config = Some({"thinking":{"type":"enabled","budget_tokens":8000}})
    And the agent processes a user prompt
    Then the JSON body sent to the script HTTP endpoint contains a thinking_config field with budget_tokens 8000

  Scenario: RhaiToolWrapper exposes Rhai-defined tool names and dispatches through default_to_internal
    Given a custom provider script that defines a tool name "read_file" with maps_to "file:read"
    When CustomProvider::create_rig_agent is invoked
    Then the resulting agent's tool set contains a tool whose name() returns "read_file"
    And calling that tool with {"file_path":"/tmp/example.txt"} routes through apply_map_tool_params and then default_to_internal returning a DispatchedToolParams::File(InternalFileParams::Read{file_path:"/tmp/example.txt", ..})
    And the dispatched ReadTool execution result is returned as the rig tool output

  Scenario: System prompt facade transform_preamble is wired in as the agent preamble
    Given a custom provider script that defines transform_preamble returning "PREFIX\\n${preamble}"
    When CustomProvider::create_rig_agent is called with preamble "user role text"
    Then the rig::agent::Agent's preamble equals "PREFIX\\nuser role text"

  Scenario: agent_loop dispatch for a custom provider routes through CustomProvider::create_rig_agent
    Given a session whose current_provider is "my-script" and a registered custom-provider config of the same name
    When the agent_loop receives a user prompt for that session
    Then the dispatch matches the custom-provider arm
    And CustomProvider::create_rig_agent is invoked with the session id, the user role preamble, and the resolved thinking_config
    And the returned rig::agent::Agent is wrapped in codelet_core::RigAgent and streamed via run_agent_stream_with_images
