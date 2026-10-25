use super::*;

#[test]
fn test_streaming_function_deserialization() -> Result<(), serde_json::Error> {
    let function: StreamingFunction =
        serde_json::from_str(r#"{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}"#)?;
    assert_eq!(function.name.as_deref(), Some("get_weather"));
    assert_eq!(
        function.arguments.as_deref(),
        Some(r#"{"location":"Paris"}"#)
    );
    Ok(())
}

#[test]
fn test_streaming_tool_call_deserialization() -> Result<(), serde_json::Error> {
    let tool_call: StreamingToolCall = serde_json::from_str(
        r#"{
        "index":0,"id":"call_abc123",
        "function":{"name":"get_weather","arguments":"{\"city\":\"London\"}"}
    }"#,
    )?;
    assert_eq!(tool_call.index, 0);
    assert_eq!(tool_call.id.as_deref(), Some("call_abc123"));
    assert_eq!(tool_call.function.name.as_deref(), Some("get_weather"));
    Ok(())
}

#[test]
fn test_streaming_tool_call_partial_deserialization() -> Result<(), serde_json::Error> {
    let tool_call: StreamingToolCall = serde_json::from_str(
        r#"{
        "index":0,"id":null,"function":{"name":null,"arguments":"Paris"}
    }"#,
    )?;
    assert_eq!(tool_call.index, 0);
    assert!(tool_call.id.is_none());
    assert!(tool_call.function.name.is_none());
    assert_eq!(tool_call.function.arguments.as_deref(), Some("Paris"));
    Ok(())
}

#[test]
fn test_streaming_delta_with_tool_calls() -> Result<(), serde_json::Error> {
    let delta: StreamingDelta = serde_json::from_str(
        r#"{
        "content":null,"tool_calls":[{
            "index":0,"id":"call_xyz","function":{"name":"search","arguments":""}
        }]
    }"#,
    )?;
    assert!(delta.content.is_none());
    assert_eq!(delta.tool_calls.len(), 1);
    assert_eq!(delta.tool_calls[0].id.as_deref(), Some("call_xyz"));
    Ok(())
}

#[test]
fn test_streaming_chunk_deserialization() -> Result<(), serde_json::Error> {
    let chunk: StreamingCompletionChunk = serde_json::from_str(
        r#"{
        "choices":[{"delta":{"content":"Hello","tool_calls":[]}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}
    }"#,
    )?;
    assert_eq!(chunk.choices.len(), 1);
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    assert!(chunk.usage.is_some());
    Ok(())
}

#[test]
fn test_streaming_chunk_with_multiple_tool_call_deltas() -> Result<(), serde_json::Error> {
    let start: StreamingCompletionChunk = serde_json::from_str(
        r#"{
        "choices":[{"delta":{"content":null,"tool_calls":[{
            "index":0,"id":"call_123","function":{"name":"get_weather","arguments":""}
        }]}}],"usage":null
    }"#,
    )?;
    let first: StreamingCompletionChunk = serde_json::from_str(
        r#"{
        "choices":[{"delta":{"content":null,"tool_calls":[{
            "index":0,"id":null,"function":{"name":null,"arguments":"{\"loc"}
        }]}}],"usage":null
    }"#,
    )?;
    let second: StreamingCompletionChunk = serde_json::from_str(
        r#"{
        "choices":[{"delta":{"content":null,"tool_calls":[{
            "index":0,"id":null,"function":{"name":null,"arguments":"ation\":\"NYC\"}"}
        }]}}],"usage":null
    }"#,
    )?;
    assert_eq!(start.choices[0].delta.tool_calls.len(), 1);
    assert_eq!(
        start.choices[0].delta.tool_calls[0]
            .function
            .name
            .as_deref(),
        Some("get_weather")
    );
    assert_eq!(first.choices[0].delta.tool_calls.len(), 1);
    assert_eq!(
        first.choices[0].delta.tool_calls[0]
            .function
            .arguments
            .as_deref(),
        Some("{\"loc")
    );
    assert_eq!(second.choices[0].delta.tool_calls.len(), 1);
    assert_eq!(
        second.choices[0].delta.tool_calls[0]
            .function
            .arguments
            .as_deref(),
        Some("ation\":\"NYC\"}")
    );
    Ok(())
}

#[tokio::test]
async fn test_streaming_usage_only_chunk_is_not_ignored() -> Result<(), Box<dyn std::error::Error>>
{
    use bytes::Bytes;
    use futures::StreamExt;

    #[derive(Clone)]
    struct MockHttpClient {
        sse_bytes: Bytes,
    }

    impl crate::http_client::HttpClientExt for MockHttpClient {
        fn send<T, U>(
            &self,
            _request: http::Request<T>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<http::Response<crate::http_client::LazyBody<U>>>,
        > + crate::wasm_compat::WasmCompatSend
        + 'static
        where
            T: Into<Bytes> + crate::wasm_compat::WasmCompatSend,
            U: From<Bytes> + crate::wasm_compat::WasmCompatSend + 'static,
        {
            std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
                http::StatusCode::NOT_IMPLEMENTED,
            )))
        }

        fn send_multipart<U>(
            &self,
            _request: http::Request<crate::http_client::MultipartForm>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<http::Response<crate::http_client::LazyBody<U>>>,
        > + crate::wasm_compat::WasmCompatSend
        + 'static
        where
            U: From<Bytes> + crate::wasm_compat::WasmCompatSend + 'static,
        {
            std::future::ready(Err(crate::http_client::Error::InvalidStatusCode(
                http::StatusCode::NOT_IMPLEMENTED,
            )))
        }

        fn send_streaming<T>(
            &self,
            _request: http::Request<T>,
        ) -> impl std::future::Future<
            Output = crate::http_client::Result<crate::http_client::StreamingResponse>,
        > + crate::wasm_compat::WasmCompatSend
        where
            T: Into<Bytes>,
        {
            let sse_bytes = self.sse_bytes.clone();
            async move {
                let byte_stream =
                    futures::stream::iter(vec![Ok::<Bytes, crate::http_client::Error>(sse_bytes)]);
                let boxed_stream: crate::http_client::sse::BoxedStream = Box::pin(byte_stream);
                http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header(reqwest::header::CONTENT_TYPE, "text/event-stream")
                    .body(boxed_stream)
                    .map_err(crate::http_client::Error::Protocol)
            }
        }
    }

    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\",\"tool_calls\":[]},\"finish_reason\":\"stop\"}],\"usage\":null}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
        "data: [DONE]\n\n",
    );
    let client = MockHttpClient {
        sse_bytes: Bytes::from(sse),
    };
    let request = http::Request::builder()
        .method("POST")
        .uri("http://localhost/v1/chat/completions")
        .body(Vec::new())?;
    let mut stream = send_compatible_streaming_request(client, request).await?;
    let mut final_usage = None;
    while let Some(chunk) = stream.next().await {
        if let streaming::StreamedAssistantContent::Final(response) = chunk? {
            final_usage = Some(response.usage);
            break;
        }
    }
    let usage = final_usage.ok_or("expected a final response with usage")?;
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.total_tokens, 15);
    Ok(())
}
