//! Streaming entrypoint for `RhaiCustomProvider` (PROV-064). Kept in a
//! separate file so `provider.rs` stays under the 300-line cap.

use std::pin::Pin;

use codelet_common::Message;
use codelet_tools::ToolDefinition;
use futures::{Stream, StreamExt};
use rhai::Dynamic;

use super::conversion::dynamic_to_json_value;
use super::provider::RhaiCustomProvider;
use super::request_bridge::request_to_rhai;
use super::stream::{open_stream, RhaiStreamProcessor, StreamChunk};
use crate::error::ProviderError;

impl RhaiCustomProvider {
    /// Invoke `build_stream_request(request)` and return its JSON body.
    ///
    /// Mirrors [`RhaiCustomProvider::invoke_build_request`] but targets
    /// the `build_stream_request` hook so providers can tweak payload
    /// shape for streaming (e.g. set `"stream": true`).
    pub async fn invoke_build_stream_request(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<serde_json::Value, ProviderError> {
        let request = request_to_rhai(messages, tools).map_err(ProviderError::from)?;
        let result = self.call_fn1_public("build_stream_request", request).await?;
        Ok(dynamic_to_json_value(&result))
    }

    /// Start a streaming completion. Returns a pinned `Stream` whose
    /// items are `Result<StreamChunk, ProviderError>`.
    ///
    /// 4xx/5xx responses yield a single `Err` and terminate the stream
    /// before any `Ok` chunk. SSE events with data `[DONE]` terminate
    /// the stream without invoking the Rhai script. Runtime errors from
    /// `parse_stream_chunk` yield a single `Err(ProviderError::Api)`.
    pub async fn complete_with_tools_streaming(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>> {
        match self.open_streaming(messages, tools).await {
            Ok(stream) => stream,
            Err(err) => {
                // Convert the single setup error into a one-item stream
                // so callers can use a uniform consumption pattern.
                Box::pin(futures::stream::once(async move { Err(err) }))
            }
        }
    }

    async fn open_streaming(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        let url = self.invoke_build_url().await?;
        let headers = self.invoke_build_headers().await?;
        let body = self.invoke_build_stream_request(messages, tools).await?;
        let provider_name = self.provider_name().to_string();

        let body_string = serde_json::to_string(&body).map_err(|e| {
            ProviderError::api(
                provider_name.clone(),
                format!("serialise stream request body: {e}"),
            )
        })?;

        let response = self
            .http_client_handle()
            .post(&url)
            .headers(headers)
            .body(body_string)
            .send()
            .await
            .map_err(|e| {
                ProviderError::api(
                    provider_name.clone(),
                    format!("streaming HTTP request failed: {e}"),
                )
            })?;

        let status = response.status().as_u16();
        let byte_stream = response.bytes_stream().map(|res| res);

        let processor = RhaiStreamProcessor::new(
            self.engine_handle(),
            self.ast_handle(),
            provider_name.clone(),
            self.config_dynamic_public(),
        );

        let this = self.clone();
        let stream = open_stream(
            processor,
            status,
            byte_stream,
            move |status, body| async move { this.invoke_map_error(status, &body).await },
        );
        Ok(stream)
    }
}

// Internal accessors. Kept in this file because they are only needed
// by the streaming entrypoint; exposing them on `RhaiCustomProvider`
// via `pub(crate)` avoids widening the public surface of provider.rs.
impl RhaiCustomProvider {
    pub(crate) fn provider_name(&self) -> &str {
        self.config_name()
    }
    pub(crate) fn config_dynamic_public(&self) -> Dynamic {
        self.config_dynamic_accessor()
    }
    pub(crate) async fn call_fn1_public(
        &self,
        fn_name: &'static str,
        arg: Dynamic,
    ) -> Result<Dynamic, ProviderError> {
        self.call_fn1_accessor(fn_name, arg).await
    }
}
