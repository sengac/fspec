@done
@provider-abstraction
@codex
@providers
@regression
@BUG-208
Feature: Codex models silently terminate: rig swallows SSE error events and AstGrepRefactor tool schema is rejected

  """
  Sanitizer lives in the patched rig-core responses_api (ResponsesToolDefinition::from / create_completion_request in rust/patches/rig-core/src/providers/openai/responses_api/mod.rs:527), so BOTH the streaming and non-streaming front doors serialize sanitized schemas — mirrors codex tools/src/json_schema.rs prepare_tool_input_schema. Minimal scope: fill properties:{} for object-typed schemas (incl. type arrays containing "object") and items default for array-typed, recursively — not a full port of codex sanitize_json_schema.
  Error-event handling also lives in the patched rig streaming.rs: add an ErrorChunk variant matched BEFORE the untagged StreamingCompletionChunk fallthrough, yielding CompletionError::ProviderError(message+code) and ending the stream — mirrors codex-api/src/sse/responses.rs:349 and the stream loop at :549/:576-580.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every tool input schema sent through the rig Responses API path MUST be sanitized: any object-typed schema (including type:["object","null"]) lacking a "properties" key gets "properties":{} inserted, recursively through properties/items/additionalProperties (mirrors codex tools/json_schema.rs ensure_default_children_for_schema_types:183-197)
  #   2. An SSE {"type":"error",...} event from the Codex backend MUST surface as a stream error (CompletionError) carrying the backend message and code — never be silently skipped (mirrors codex-api sse/responses.rs:349 + stream loop sending Err(error))
  #   3. Sanitize at the rig responses_api layer (ResponsesToolDefinition::from in the patched rig-core), covering both Codex and plain OpenAI responses models via one shared code path — confirmed by user.
  #
  # EXAMPLES:
  #   1. Unit: an SSE stream containing {"type":"error","error":{"message":"Invalid schema for function 'AstGrepRefactor'...","code":"invalid_function_parameters"}} yields a stream Err containing that message instead of completing with zero chunks
  #   2. Live: gpt-6-astra full agent path (create_rig_agent with all ~25 tools + multi_turn) produces non-empty final text — the existing red test live_gpt6_astra_full_agent_path turns green
  #   3. Unit: a tool schema {"type":["object","null"],"additionalProperties":{...}} (what schemars 0.8 emits for Option<HashMap<String,Transform>> in AstGrepRefactor) is sanitized so every object-typed branch gains "properties":{} before the request is serialized
  #   4. TUI: if the backend still rejects a request for another reason, the conversation ends with a visible provider error containing the backend message (e.g. invalid_function_parameters), not a silent empty turn
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the schema sanitizer apply to all Responses-API providers (OpenAI + Codex) at the rig layer, or only to the Codex provider path?
  #   A: Sanitize at the rig responses_api layer (ResponsesToolDefinition::from in the patched rig-core), covering both Codex and plain OpenAI responses models via one shared code path — confirmed by user.
  #
  # ========================================

  Background: User Story
    As a agent user
    I want to converse with Codex-backend models (gpt-6-astra) with tools enabled
    So that the conversation either works or fails with a clear error, instead of silently terminating with no output

  Scenario: Backend error events surface as stream errors instead of silent completion
    Given an SSE response stream from the Responses API backend containing an error event:
      """
      {"type":"error","error":{"message":"Invalid schema for function 'AstGrepRefactor': At schema path /properties/transforms/type/0, object schema must define properties","code":"invalid_function_parameters"}}
      """
    When the stream is consumed through the rig Responses API streaming path
    Then the stream yields an error containing the backend message "Invalid schema for function 'AstGrepRefactor'"
    And the stream error carries the backend code "invalid_function_parameters"
    And the stream does not complete with a silent empty final response

  Scenario: Object-typed tool schemas are sanitized before the request is sent
    Given a tool whose input schema contains an object-typed branch without a "properties" key:
      """
      {"type":["object","null"],"additionalProperties":{"type":"object","properties":{"source":{"type":"string"}}}}
      """
    When the tool definition is converted for a Responses API request
    Then every object-typed branch of the schema has "properties" present (an empty object when no properties are declared)
    And the "additionalProperties" subschema is preserved and recursively sanitized
    And the sanitized schema is what gets serialized into the request body

  Scenario: Array-typed schemas without items get a permissive default
    Given a tool input schema containing {"type":"array"} with no "items" key
    When the tool definition is converted for a Responses API request
    Then the array schema gains an "items" default so the backend's strict validator accepts it

  Scenario: Tool definitions are registered with strict false like the Codex client
    Given a tool definition with a type-less (description-only) property schema exists
    When the tool definition is converted for a Responses API request
    Then the serialized tool payload carries "strict": false (mirrors codex tool_definition_to_responses_api_tool)
    And the parameters schema is still sanitized (properties/items defaults filled) so the schema remains well-formed

