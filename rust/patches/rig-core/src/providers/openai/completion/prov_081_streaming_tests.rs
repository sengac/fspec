// ================================================================
// PROV-081: `reasoning` / `reasoning_content` capture (streaming)
//
// Feature: spec/features/openai-provider-reasoning-tokens.feature
//
// The four streaming scenarios, split from prov_081_tests.rs to keep
// each test file under 300 lines (COV-075).
// ================================================================
use super::*;
use crate::streaming as core_streaming;

// ------------------------------------------------------------
// Streaming helpers (reused across the four streaming scenarios)
// ------------------------------------------------------------

/// Mock HTTP client that serves a canned SSE payload to the streaming decoder.
#[derive(Clone)]
struct MockSseClient {
    sse_bytes: bytes::Bytes,
}

impl crate::http_client::HttpClientExt for MockSseClient {
    fn send<T, U>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = crate::http_client::Result<http::Response<crate::http_client::LazyBody<U>>>,
    > + crate::wasm_compat::WasmCompatSend
    + 'static
    where
        T: Into<bytes::Bytes>,
        T: crate::wasm_compat::WasmCompatSend,
        U: From<bytes::Bytes>,
        U: crate::wasm_compat::WasmCompatSend + 'static,
    {
        std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_multipart<U>(
        &self,
        _req: http::Request<crate::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = crate::http_client::Result<http::Response<crate::http_client::LazyBody<U>>>,
    > + crate::wasm_compat::WasmCompatSend
    + 'static
    where
        U: From<bytes::Bytes>,
        U: crate::wasm_compat::WasmCompatSend + 'static,
    {
        std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_streaming<T>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = crate::http_client::Result<crate::http_client::StreamingResponse>,
    > + crate::wasm_compat::WasmCompatSend
    where
        T: Into<bytes::Bytes>,
    {
        let sse_bytes = self.sse_bytes.clone();
        async move {
            let byte_stream = futures::stream::iter(vec![Ok::<
                bytes::Bytes,
                crate::http_client::Error,
            >(sse_bytes)]);
            let boxed_stream: crate::http_client::sse::BoxedStream = Box::pin(byte_stream);

            http::Response::builder()
                .status(http::StatusCode::OK)
                .header(reqwest::header::CONTENT_TYPE, "text/event-stream")
                .body(boxed_stream)
                .map_err(crate::http_client::Error::Protocol)
        }
    }
}

/// Drive an SSE payload through `send_compatible_streaming_request` and collect
/// every emitted `StreamedAssistantContent` variant for assertions.
async fn collect_streamed_contents(
    sse: &'static str,
) -> Vec<core_streaming::StreamedAssistantContent<streaming::StreamingCompletionResponse>> {
    use futures::StreamExt;

    let client = MockSseClient {
        sse_bytes: bytes::Bytes::from(sse),
    };

    let req = http::Request::builder()
        .method("POST")
        .uri("http://localhost/v1/chat/completions")
        .body(Vec::new())
        .expect("build mock request");

    let mut stream = streaming::send_compatible_streaming_request(client, req)
        .await
        .expect("stream init should succeed");

    let mut out = Vec::new();
    while let Some(chunk) = stream.next().await {
        let value = chunk.expect("each chunk should be Ok");
        let is_final = matches!(value, core_streaming::StreamedAssistantContent::Final(_));
        out.push(value);
        if is_final {
            break;
        }
    }
    out
}

/// Build the JSON body that would be POSTed to `/chat/completions`
/// for a plain caller request (mirrors what `completion::CompletionModel::completion`
/// does internally, minus the HTTP send).

// ------------------------------------------------------------
// Scenario 1: Streaming — vLLM-native `reasoning` field
// ------------------------------------------------------------
#[tokio::test]
async fn prov_081_streaming_vllm_reasoning_field_surfaces_as_reasoning_delta() {
    // @step Given a caller streams a chat completion through the OpenAI provider
    // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning":"Let me analyse..."}}]}
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning\":\"Let me analyse...\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n",
    );

    // @step When the provider decodes the chunk
    let events = collect_streamed_contents(sse).await;

    // @step Then the caller receives a ReasoningDelta whose reasoning text equals "Let me analyse..."
    let reasoning_texts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            core_streaming::StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                Some(reasoning.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        reasoning_texts,
        vec!["Let me analyse...".to_string()],
        "expected exactly one ReasoningDelta carrying the vLLM `reasoning` field content"
    );

    // @step And the caller does not receive any content delta for that chunk
    for e in &events {
        if let core_streaming::StreamedAssistantContent::Text(t) = e {
            panic!("unexpected content delta: {:?}", t);
        }
    }

    // @step And the chunk does not produce a decode error
    // (collect_streamed_contents unwraps each chunk — a decode error would have panicked above)
}

// ------------------------------------------------------------
// Scenario 2: Streaming — Z.AI/GLM `reasoning_content` field
// ------------------------------------------------------------
#[tokio::test]
async fn prov_081_streaming_glm_reasoning_content_field_surfaces_as_reasoning_delta() {
    // @step Given a caller streams a chat completion through the OpenAI provider
    // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking...\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n",
    );

    // @step When the provider decodes the chunk
    let events = collect_streamed_contents(sse).await;

    // @step Then the caller receives a ReasoningDelta whose reasoning text equals "thinking..."
    let reasoning_texts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            core_streaming::StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                Some(reasoning.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        reasoning_texts,
        vec!["thinking...".to_string()],
        "expected exactly one ReasoningDelta carrying the GLM `reasoning_content` field content"
    );

    // @step And the caller does not receive any content delta for that chunk
    for e in &events {
        if let core_streaming::StreamedAssistantContent::Text(t) = e {
            panic!("unexpected content delta: {:?}", t);
        }
    }
}

// ------------------------------------------------------------
// Scenario 3: Streaming — concatenate when both fields present
// ------------------------------------------------------------
#[tokio::test]
async fn prov_081_streaming_concatenates_reasoning_when_both_fields_present() {
    // @step Given a caller streams a chat completion through the OpenAI provider
    // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"B","reasoning":"A"}}]}
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"B\",\"reasoning\":\"A\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n",
    );

    // @step When the provider decodes the chunk
    let events = collect_streamed_contents(sse).await;

    // @step Then the caller receives reasoning text equal to "BA" with reasoning_content concatenated first and reasoning appended
    let combined: String = events
        .iter()
        .filter_map(|e| match e {
            core_streaming::StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                Some(reasoning.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(
        combined, "BA",
        "expected reasoning output to be exactly 'BA' \
         (reasoning_content='B' concatenated first, reasoning='A' appended), \
         got {combined:?}"
    );

    // @step And no reasoning character from either source field is dropped
    assert_eq!(
        combined.chars().filter(|c| *c == 'A').count(),
        1,
        "character 'A' must appear exactly once (not dropped, not duplicated)"
    );
    assert_eq!(
        combined.chars().filter(|c| *c == 'B').count(),
        1,
        "character 'B' must appear exactly once (not dropped, not duplicated)"
    );
}

// ------------------------------------------------------------
// Scenario 4: Streaming — regression: plain content still works
// ------------------------------------------------------------
#[tokio::test]
async fn prov_081_streaming_content_chunk_passes_through_unchanged() {
    // @step Given a caller streams a chat completion through the OpenAI provider
    // @step And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"content":"hello"}}]}
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n\n",
        "data: [DONE]\n\n",
    );

    // @step When the provider decodes the chunk
    let events = collect_streamed_contents(sse).await;

    // @step Then the caller receives a content delta whose text equals "hello"
    let content_texts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            core_streaming::StreamedAssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(content_texts, vec!["hello".to_string()]);

    // @step And the caller does not receive any ReasoningDelta
    for e in &events {
        if let core_streaming::StreamedAssistantContent::ReasoningDelta { reasoning, .. } = e {
            panic!("unexpected ReasoningDelta: {:?}", reasoning);
        }
    }

    // @step And the chunk does not produce a decode error
    // (collect_streamed_contents unwraps each chunk — a decode error would have panicked above)
}
