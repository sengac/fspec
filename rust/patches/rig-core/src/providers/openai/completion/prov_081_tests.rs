// ================================================================
// PROV-081: vLLM-native `reasoning` field capture (non-streaming)
//
// Feature: spec/features/openai-provider-reasoning-tokens.feature
//
// These tests validate that the OpenAI provider recognizes the
// `reasoning` field emitted by vLLM (Qwen3 reasoning-parser output)
// in addition to the Z.AI/GLM `reasoning_content` field on
// non-streaming assistant messages, AND that the outgoing request
// body never emits reasoning-suppression keys.
// ================================================================
use super::*;
use crate::OneOrMany;
use crate::completion;

fn build_request_body_json() -> serde_json::Value {
    // A minimal caller-built CompletionRequest (no reasoning-suppression flags).
    let core_req = crate::completion::CompletionRequest {
        preamble: None,
        chat_history: OneOrMany::one(crate::completion::Message::User {
            content: OneOrMany::one(crate::message::UserContent::text("hi")),
        }),
        documents: Vec::new(),
        tools: Vec::new(),
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
    };

    let wire_req = CompletionRequest::try_from(OpenAIRequestParams {
        model: "qwen".to_string(),
        request: core_req,
        strict_tools: false,
        tool_result_array_content: false,
    })
    .expect("building request should succeed");

    serde_json::to_value(&wire_req).expect("serializing request should succeed")
}

/// Extract reasoning text from a CompletionResponse's choice (OneOrMany<AssistantContent>).
fn extract_reasoning_texts(
    response: &completion::CompletionResponse<CompletionResponse>,
) -> Vec<String> {
    response
        .choice
        .iter()
        .filter_map(|c| match c {
            completion::AssistantContent::Reasoning(r) => Some(r.reasoning.join("")),
            _ => None,
        })
        .collect()
}

/// Extract text content from a CompletionResponse's choice.
fn extract_text_contents(
    response: &completion::CompletionResponse<CompletionResponse>,
) -> Vec<String> {
    response
        .choice
        .iter()
        .filter_map(|c| match c {
            completion::AssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn prov_081_non_streaming_vllm_reasoning_field_surfaces_reasoning_and_content() {
    // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
    // @step And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning":"analysis","content":"answer"}
    let body = serde_json::json!({
        "id": "resp-1",
        "object": "chat.completion",
        "created": 1,
        "model": "qwen",
        "system_fingerprint": null,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "reasoning": "analysis",
                "content": "answer"
            },
            "logprobs": null,
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens":10,"completion_tokens":2,"total_tokens":12}
    });

    let raw: CompletionResponse =
        serde_json::from_value(body).expect("deserialize raw provider response");

    // @step When the provider decodes the response
    let response: completion::CompletionResponse<CompletionResponse> = raw
        .try_into()
        .expect("convert into caller-visible response");

    // @step Then the caller-visible CompletionResponse exposes reasoning text "analysis"
    let reasoning = extract_reasoning_texts(&response);
    assert_eq!(
        reasoning,
        vec!["analysis".to_string()],
        "expected reasoning text 'analysis' surfaced from vLLM `reasoning` field"
    );

    // @step And the caller-visible CompletionResponse exposes content text "answer"
    let content = extract_text_contents(&response);
    assert_eq!(
        content,
        vec!["answer".to_string()],
        "expected content text 'answer' surfaced from `content` field"
    );
}

#[test]
fn prov_081_non_streaming_glm_reasoning_content_field_surfaces_reasoning_and_content() {
    // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
    // @step And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning_content":"analysis","content":"answer"}
    let body = serde_json::json!({
        "id": "resp-2",
        "object": "chat.completion",
        "created": 1,
        "model": "glm-4",
        "system_fingerprint": null,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "reasoning_content": "analysis",
                "content": "answer"
            },
            "logprobs": null,
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens":10,"completion_tokens":2,"total_tokens":12}
    });

    let raw: CompletionResponse =
        serde_json::from_value(body).expect("deserialize raw provider response");

    // @step When the provider decodes the response
    let response: completion::CompletionResponse<CompletionResponse> = raw
        .try_into()
        .expect("convert into caller-visible response");

    // @step Then the caller-visible CompletionResponse exposes reasoning text "analysis"
    let reasoning = extract_reasoning_texts(&response);
    assert_eq!(
        reasoning,
        vec!["analysis".to_string()],
        "expected reasoning text 'analysis' surfaced from GLM `reasoning_content` field"
    );

    // @step And the caller-visible CompletionResponse exposes content text "answer"
    let content = extract_text_contents(&response);
    assert_eq!(content, vec!["answer".to_string()]);
}

#[test]
fn prov_081_non_streaming_without_reasoning_field_still_works() {
    // @step Given a caller issues a non-streaming chat completion through the OpenAI provider
    // @step And the upstream server returns a response whose assistant message is {"role":"assistant","content":"answer"}
    let body = serde_json::json!({
        "id": "resp-3",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-4o",
        "system_fingerprint": null,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "answer"
            },
            "logprobs": null,
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens":10,"completion_tokens":2,"total_tokens":12}
    });

    // @step When the provider decodes the response
    let raw: CompletionResponse = serde_json::from_value(body)
        .expect("plain response must still deserialize with no reasoning field");
    let response: completion::CompletionResponse<CompletionResponse> = raw
        .try_into()
        .expect("convert into caller-visible response");

    // @step Then the caller-visible CompletionResponse exposes content text "answer"
    let content = extract_text_contents(&response);
    assert_eq!(content, vec!["answer".to_string()]);

    // @step And the caller-visible CompletionResponse does not expose any reasoning text
    let reasoning = extract_reasoning_texts(&response);
    assert!(
        reasoning.is_empty(),
        "expected no reasoning text surfaced, got {reasoning:?}"
    );

    // @step And the response does not produce a decode error
    // (deserialize + try_into above both unwrapped — any error would have panicked)
}

#[test]
fn prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys() {
    // @step Given a caller builds a chat completion request through the OpenAI provider without explicitly setting any reasoning-suppression flag
    // @step When the provider serializes the request body that would be POSTed to /chat/completions
    let body = build_request_body_json();

    // @step Then the serialized body does not contain a top-level key named "include_reasoning"
    let obj = body
        .as_object()
        .expect("serialized body should be a JSON object");
    assert!(
        !obj.contains_key("include_reasoning"),
        "request body must NOT carry `include_reasoning` (vLLM strips reasoning when false). Body: {body}"
    );

    // @step And the serialized body does not contain a "chat_template_kwargs.enable_thinking" key path
    if let Some(chat_template_kwargs) = obj.get("chat_template_kwargs")
        && let Some(template_options) = chat_template_kwargs.as_object()
    {
        assert!(
            !template_options.contains_key("enable_thinking"),
            "request body must NOT carry `chat_template_kwargs.enable_thinking` (Qwen3 template-level kill-switch). Body: {body}"
        );
    }
}
