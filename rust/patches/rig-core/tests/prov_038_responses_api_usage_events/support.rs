use rig::providers::openai::responses_api::{
    InputTokensDetails, OutputTokensDetails, ResponsesUsage,
};

// =============================================================================
// Shared mock HTTP client for integration tests
// =============================================================================

#[derive(Clone, Debug, Default)]
pub(super) struct MockHttpClient {
    sse_bytes: bytes::Bytes,
}

pub(super) fn mock_http_client(sse_bytes: bytes::Bytes) -> MockHttpClient {
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

/// Helper: build a ResponsesUsage with full details
pub(super) fn make_usage_with_details(
    input: u64,
    output: u64,
    total: u64,
    cached: u64,
    reasoning: u64,
) -> ResponsesUsage {
    ResponsesUsage {
        input_tokens: input,
        input_tokens_details: Some(InputTokensDetails {
            cached_tokens: cached,
        }),
        output_tokens: output,
        output_tokens_details: Some(OutputTokensDetails {
            reasoning_tokens: reasoning,
        }),
        total_tokens: total,
    }
}

/// Helper: build a ResponsesUsage with no input_tokens_details
pub(super) fn make_usage_without_details(input: u64, output: u64, total: u64) -> ResponsesUsage {
    ResponsesUsage {
        input_tokens: input,
        input_tokens_details: None,
        output_tokens: output,
        output_tokens_details: Some(OutputTokensDetails {
            reasoning_tokens: 0,
        }),
        total_tokens: total,
    }
}
