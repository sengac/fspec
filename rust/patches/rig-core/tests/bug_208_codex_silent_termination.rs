//! Feature: spec/features/codex-models-silently-terminate-rig-swallows-sse-error-events-and-astgreprefactor-tool-schema-is-rejected.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios (scenarios 1-3; scenario 4 is the
//! live test `live_gpt6_astra_full_agent_path` in
//! rust/providers/tests/codex_gpt6_astra_live_test.rs).
//!
//! BUG-208: the Codex backend (chatgpt.com/backend-api/codex/responses) validates
//! every tool input schema and, on the first offender, replies with a single SSE
//! {"type":"error",...} event over HTTP 200. rig's untagged StreamingCompletionChunk
//! cannot deserialize that event and silently skips it, so the conversation ends
//! with an empty FinalResponse and no error. Two defects, two fixes:
//!   1. streaming.rs must surface error events as CompletionError
//!      (mirrors codex codex-api/src/sse/responses.rs:349 + stream loop :576-580).
//!   2. sanitize_node must give object-typed schemas a "properties" key and
//!      array-typed schemas an "items" default
//!      (mirrors codex tools/src/json_schema.rs ensure_default_children_for_schema_types:183-197).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bytes::Bytes;
use futures::StreamExt;

// =============================================================================
// Shared mock HTTP client (same pattern as prov_038_responses_api_usage_events.rs)
// =============================================================================

#[derive(Clone, Debug, Default)]
struct MockHttpClient {
    sse_bytes: bytes::Bytes,
}

fn mock_http_client(sse_bytes: bytes::Bytes) -> MockHttpClient {
    MockHttpClient { sse_bytes }
}

impl rig::http_client::HttpClientExt for MockHttpClient {
    fn send<T, U>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
    + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_multipart<U>(
        &self,
        _req: http::Request<rig::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
    + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_streaming<T>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<rig::http_client::StreamingResponse>,
    > + Send
    where
        T: Into<bytes::Bytes>,
    {
        let sse_bytes = self.sse_bytes.clone();
        async move {
            let byte_stream =
                futures::stream::iter(vec![Ok::<bytes::Bytes, rig::http_client::Error>(sse_bytes)]);
            let boxed_stream: rig::http_client::sse::BoxedStream = Box::pin(byte_stream);

            http::Response::builder()
                .status(http::StatusCode::OK)
                .header(reqwest::header::CONTENT_TYPE, "text/event-stream")
                .body(boxed_stream)
                .map_err(rig::http_client::Error::Protocol)
        }
    }
}

fn responses_client(sse: &str) -> rig::providers::openai::Client<MockHttpClient> {
    rig::providers::openai::Client::<MockHttpClient>::builder()
        .api_key("test-key")
        .base_url("http://test.local/v1")
        .http_client(mock_http_client(Bytes::from(sse.to_string())))
        .build()
        .expect("should build client")
}

fn minimal_request() -> rig::completion::CompletionRequest {
    rig::completion::CompletionRequest {
        preamble: Some("Test".to_string()),
        chat_history: rig::OneOrMany::one(rig::completion::Message::user("Hello")),
        tools: vec![],
        documents: vec![],
        temperature: None,
        max_tokens: None,
        additional_params: None,
        tool_choice: None,
    }
}

// =============================================================================
// Scenario: Backend error events surface as stream errors instead of silent completion
// =============================================================================

#[tokio::test]
async fn scenario_backend_error_events_surface_as_stream_errors_instead_of_silent_completion() {
    use rig::client::CompletionClient;
    use rig::completion::CompletionModel;

    // @step Given an SSE response stream from the Responses API backend containing an error event:
    // (exact payload captured from the live Codex backend in the BUG-208 analysis:
    //  a single SSE event over HTTP 200, nothing else)
    let sse = concat!(
        "event: error\n",
        "data: {\"type\":\"error\",\"error\":{\"message\":\"Invalid schema for function 'AstGrepRefactor': At schema path /properties/transforms/type/0, object schema must define properties\",\"code\":\"invalid_function_parameters\"}}\n\n",
    );

    let client = responses_client(sse);
    let model = client.completion_model("gpt-6-astra");

    // @step When the stream is consumed through the rig Responses API streaming path
    let mut stream: rig::streaming::StreamingCompletionResponse<
        rig::providers::openai::responses_api::streaming::StreamingCompletionResponse,
    > = model
        .stream(minimal_request())
        .await
        .expect("stream should start (HTTP 200)");

    let mut saw_error: Option<String> = None;
    let mut saw_final_response = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(rig::streaming::StreamedAssistantContent::Final(_)) => {
                saw_final_response = true;
            }
            Err(e) => {
                saw_error = Some(format!("{e}"));
            }
            _ => {}
        }
    }

    // @step Then the stream yields an error containing the backend message "Invalid schema for function 'AstGrepRefactor'"
    let err_msg = saw_error.unwrap_or_else(|| {
        panic!(
            "stream must yield an Err for an SSE error event, got final_response={saw_final_response} instead (silent-termination bug)"
        )
    });
    assert!(
        err_msg.contains("Invalid schema for function 'AstGrepRefactor'"),
        "stream error must contain the backend message, got: {err_msg}"
    );

    // @step And the stream error carries the backend code "invalid_function_parameters"
    assert!(
        err_msg.contains("invalid_function_parameters"),
        "stream error must carry the backend error code, got: {err_msg}"
    );

    // @step And the stream does not complete with a silent empty final response
    assert!(
        !saw_final_response,
        "stream must not emit a silent empty FinalResponse after a backend error event"
    );
}

// =============================================================================
// Scenario: Object-typed tool schemas are sanitized before the request is sent
// =============================================================================

#[test]
fn scenario_object_typed_tool_schemas_are_sanitized_before_the_request_is_sent() {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    // @step Given a tool whose input schema contains an object-typed branch without a "properties" key:
    // (exactly what schemars 0.8 emits for `Option<HashMap<String, Transform>>`
    //  in the AstGrepRefactor tool — the offender the Codex backend rejects)
    let tool = ToolDefinition {
        name: "AstGrepRefactor".to_string(),
        description: "AST-based refactor".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "transforms": {
                    "type": ["object", "null"],
                    "additionalProperties": {
                        "type": "object",
                        "properties": {
                            "source": { "type": "string" }
                        }
                    }
                }
            }
        }),
    };

    // @step When the tool definition is converted for a Responses API request
    let def = ResponsesToolDefinition::from(tool);
    let transforms = &def.parameters["properties"]["transforms"];

    // @step Then every object-typed branch of the schema has "properties" present (an empty object when no properties are declared)
    assert_eq!(
        transforms["type"],
        serde_json::json!(["object", "null"]),
        "the type array must be preserved as-is"
    );
    assert_eq!(
        transforms.get("properties"),
        Some(&serde_json::json!({})),
        "object-typed branch (type:[\"object\",\"null\"]) must gain \"properties\":{{}} — got: {transforms}"
    );

    // @step And the "additionalProperties" subschema is preserved and recursively sanitized
    let ap = &transforms["additionalProperties"];
    assert_eq!(
        ap["type"],
        serde_json::json!("object"),
        "additionalProperties subschema must be preserved"
    );
    assert_eq!(
        ap["properties"]["source"]["type"],
        serde_json::json!("string"),
        "inner property schemas must survive sanitization"
    );
    // Recursive sanitization: the existing sanitizer marks all fields required
    // and closes objects — it must have run inside additionalProperties too.
    assert_eq!(
        ap["required"],
        serde_json::json!(["source"]),
        "additionalProperties subschema must be recursively sanitized (required filled) — got: {ap}"
    );

    // @step And the sanitized schema is what gets serialized into the request body
    let serialized = serde_json::to_value(&def).expect("tool definition must serialize");
    assert_eq!(
        serialized["parameters"]["properties"]["transforms"].get("properties"),
        Some(&serde_json::json!({})),
        "the serialized request payload must contain the sanitized schema"
    );

    // @step And the sanitized schema is what gets serialized into the request body
    // (end-to-end: the request the model would actually send carries the sanitized schema)
    let req = build_request_with_tool();
    let body = serde_json::to_value(&req).expect("request must serialize");
    let sent_transforms = &body["tools"][0]["parameters"]["properties"]["transforms"];
    assert_eq!(
        sent_transforms.get("properties"),
        Some(&serde_json::json!({})),
        "the request body sent to the backend must contain the sanitized schema — got: {sent_transforms}"
    );
}

/// Build a Responses API request carrying the offending AstGrepRefactor tool,
/// exactly as `ResponsesCompletionModel::create_completion_request` would.
fn build_request_with_tool() -> rig::providers::openai::responses_api::CompletionRequest {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    let tool = ToolDefinition {
        name: "AstGrepRefactor".to_string(),
        description: "AST-based refactor".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "transforms": {
                    "type": ["object", "null"],
                    "additionalProperties": {
                        "type": "object",
                        "properties": {
                            "source": { "type": "string" }
                        }
                    }
                }
            }
        }),
    };

    let mut req = rig::providers::openai::responses_api::CompletionRequest::try_from((
        "gpt-6-astra".to_string(),
        minimal_request(),
    ))
    .expect("request conversion should succeed");
    req.tools = vec![ResponsesToolDefinition::from(tool)];
    req
}

// =============================================================================
// Scenario: Array-typed schemas without items get a permissive default
// =============================================================================

#[test]
fn scenario_array_typed_schemas_without_items_get_a_permissive_default() {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    // @step Given a tool input schema containing {"type":"array"} with no "items" key
    let tool = ToolDefinition {
        name: "some_tool".to_string(),
        description: "Tool with a bare array field".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "tags": { "type": "array" }
            }
        }),
    };

    // @step When the tool definition is converted for a Responses API request
    let def = ResponsesToolDefinition::from(tool);
    let tags = &def.parameters["properties"]["tags"];

    // @step Then the array schema gains an "items" default so the backend's strict validator accepts it
    // (mirrors codex tools/json_schema.rs ensure_default_children_for_schema_types:
    //  missing items default to {"type":"string"})
    assert_eq!(
        tags.get("items"),
        Some(&serde_json::json!({"type": "string"})),
        "array-typed schema must gain an items default — got: {tags}"
    );
}

// =============================================================================
// Live-discovered follow-up offender (surfaced by live_gpt6_astra_full_agent_path
// after the AstGrepRefactor schema was fixed): the Decision tool's
// `questions` map carries minProperties/maxProperties from schemars'
// HashMap bounds, which the backend rejects:
//   "At schema path /properties/questions, keyword \"minProperties\" is not
//    supported for strict schemas" (code: invalid_function_parameters).
// Same rule as the two scenarios above: the request must survive backend
// validation — unsupported strict-schema keywords are stripped.
// =============================================================================

#[test]
fn object_property_bounds_keywords_are_stripped_for_strict_schema_compatibility() {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    // @step Given a tool whose object-typed field carries minProperties/maxProperties
    // (exactly what schemars 0.8 emits for `HashMap<String, Question>` in the Decision tool)
    let tool = ToolDefinition {
        name: "Decision".to_string(),
        description: "Ask the decision engine typed questions".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "object",
                    "minProperties": 1,
                    "maxProperties": 100,
                    "additionalProperties": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string" }
                        }
                    }
                }
            }
        }),
    };

    // @step When the tool definition is converted for a Responses API request
    let def = ResponsesToolDefinition::from(tool);
    let questions = &def.parameters["properties"]["questions"];

    // @step Then the strict-schema-incompatible bounds keywords are removed
    assert!(
        questions.get("minProperties").is_none(),
        "minProperties must be stripped (not supported for strict schemas) — got: {questions}"
    );
    assert!(
        questions.get("maxProperties").is_none(),
        "maxProperties must be stripped (not supported for strict schemas) — got: {questions}"
    );

    // @step And the rest of the schema is preserved and still sanitized
    assert_eq!(
        questions.get("properties"),
        Some(&serde_json::json!({})),
        "object-typed branch must still gain properties:{{}} — got: {questions}"
    );
    assert_eq!(
        questions["additionalProperties"]["properties"]["type"]["type"],
        serde_json::json!("string"),
        "inner subschemas must survive keyword stripping"
    );
}

// Second live-discovered offender (surfaced after the minProperties fix):
//   "Invalid schema for function 'Decision': At schema path
//    /properties/questions/additionalProperties/properties/criteria, schema must
//    specify a type" (code: invalid_function_parameters).
// The Decision tool declares `criteria` with only a description (its shape
// depends on the question type). Mirrors codex-rs sanitize_json_schema: infer a
// type from structural hints; with no hints, reduce the node to `{}` (accept-all).
#[test]
fn type_less_schema_nodes_are_typed_or_reduced_to_accept_all() {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    // @step Given a tool with a description-only property and a hint-only property
    let tool = ToolDefinition {
        name: "Decision".to_string(),
        description: "Ask the decision engine typed questions".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "criteria": {
                    "description": "choice: object qid->text. score: array. noul: object."
                },
                "labels": {
                    "enum": ["a", "b"]
                }
            }
        }),
    };

    // @step When the tool definition is converted for a Responses API request
    let def = ResponsesToolDefinition::from(tool);

    // @step Then the description-only node is reduced to the accept-all schema
    let criteria = &def.parameters["properties"]["criteria"];
    assert_eq!(
        criteria,
        &serde_json::json!({}),
        "type-less, hint-less node must be reduced to {{}} (accept-all) — got: {criteria}"
    );

    // @step And a node with structural hints gains an inferred type
    let labels = &def.parameters["properties"]["labels"];
    assert_eq!(
        labels["type"],
        serde_json::json!("string"),
        "enum-only node must infer type:string — got: {labels}"
    );
}

// =============================================================================
// Scenario: Tool definitions are registered with strict false like the Codex client
//
// Third live-discovered offender: even after the sanitizer reduced the Decision
// tool's `criteria` node to the accept-all schema `{}`, the backend still
// answered "At schema path /properties/questions/additionalProperties/properties/criteria,
// schema must specify a type" — because rig sends `strict: true` on every tool,
// which activates the backend's strict-schema validator. The Codex client itself
// registers dynamic tools with strict:false (codex-rs
// tools/src/responses_api.rs:168 tool_definition_to_responses_api_tool), and
// that is why the same backend accepts codex's schemas. Non-strict registration
// accepts the accept-all `{}` schema; strict registration does not.
// =============================================================================

#[test]
fn scenario_tool_definitions_are_registered_with_strict_false_like_the_codex_client() {
    use rig::completion::ToolDefinition;
    use rig::providers::openai::responses_api::ResponsesToolDefinition;

    // @step Given a tool definition with a type-less (description-only) property schema exists
    let tool = ToolDefinition {
        name: "Decision".to_string(),
        description: "Ask the decision engine typed questions".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "criteria": {
                    "description": "choice: object qid->text. score: array. noul: object."
                }
            }
        }),
    };

    // @step When the tool definition is converted for a Responses API request
    let def = ResponsesToolDefinition::from(tool);

    // @step Then the serialized tool payload carries "strict": false (mirrors codex tool_definition_to_responses_api_tool)
    let serialized = serde_json::to_value(&def).expect("tool definition must serialize");
    assert_eq!(
        serialized["strict"],
        serde_json::json!(false),
        "tools must be registered non-strict like the Codex client — got: {serialized}"
    );

    // @step And the parameters schema is still sanitized (properties/items defaults filled) so the schema remains well-formed
    assert_eq!(
        def.parameters.get("properties").map(|p| p
            .get("criteria")
            .cloned()
            .unwrap_or(serde_json::Value::Null)),
        Some(serde_json::json!({})),
        "the type-less criteria node must still be reduced to accept-all {{}} — got: {}",
        def.parameters
    );
    assert_eq!(
        def.parameters.get("required"),
        Some(&serde_json::json!(["criteria"])),
        "the sanitizer must still run (required filled) — got: {}",
        def.parameters
    );
}
