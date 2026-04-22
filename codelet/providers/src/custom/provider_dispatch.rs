//! LLM-call / HTTP dispatch helpers for [`RhaiCustomProvider`].
//!
//! Extracted from `provider.rs` to keep that file under the 300-line
//! threshold mandated by the project coding standards. All functions
//! here are methods on `RhaiCustomProvider` via a secondary `impl`
//! block; the split is purely for file-size hygiene — no behaviour
//! changes.

use codelet_common::Message;
use codelet_tools::ToolDefinition;
use reqwest::header::HeaderMap;
use rhai::Dynamic;

use super::conversion::{dynamic_to_json_value, json_value_to_dynamic};
use super::error_mapping::dynamic_to_provider_error;
use super::http::dynamic_to_header_map;
use super::log_helpers::{truncate_json_preview, truncate_str};
use super::provider::RhaiCustomProvider;
use super::request_bridge::request_to_rhai;
use super::response_bridge::rhai_to_completion_response;
use super::rhai_call::{call_fn1, call_fn2};
use super::stream::StreamUsage;
use crate::error::ProviderError;
use crate::{CompletionResponse, LlmProvider};

impl RhaiCustomProvider {
    /// Offload a 1-arg Rhai call to `spawn_blocking`.
    pub(crate) async fn call_fn1(
        &self,
        fn_name: &'static str,
        arg: Dynamic,
    ) -> Result<Dynamic, ProviderError> {
        call_fn1(
            self.engine_handle(),
            self.ast_handle(),
            self.config_name().to_string(),
            fn_name,
            arg,
        )
        .await
    }

    /// Offload a 2-arg Rhai call to `spawn_blocking`.
    pub(crate) async fn call_fn2(
        &self,
        fn_name: &'static str,
        arg1: Dynamic,
        arg2: Dynamic,
    ) -> Result<Dynamic, ProviderError> {
        call_fn2(
            self.engine_handle(),
            self.ast_handle(),
            self.config_name().to_string(),
            fn_name,
            arg1,
            arg2,
        )
        .await
    }

    /// Invoke `build_url(config)` and return the resulting URL string.
    pub async fn invoke_build_url(&self) -> Result<String, ProviderError> {
        tracing::warn!(
            provider = %self.config_name(),
            model = %self.model(),
            "[rhai-dispatch] invoke_build_url: calling Rhai build_url(config)"
        );
        let result = self.call_fn1("build_url", self.config_dynamic()).await?;
        let url = result.into_string().map_err(|typ| {
            ProviderError::api(
                self.config_name().to_string(),
                format!("build_url must return a string, got {typ}"),
            )
        })?;
        tracing::warn!(
            provider = %self.config_name(),
            url = %url,
            "[rhai-dispatch] invoke_build_url: Rhai returned URL"
        );
        Ok(url)
    }

    /// Invoke `build_headers(config)` and return a `HeaderMap`.
    pub async fn invoke_build_headers(&self) -> Result<HeaderMap, ProviderError> {
        tracing::warn!(
            provider = %self.config_name(),
            "[rhai-dispatch] invoke_build_headers: calling Rhai build_headers(config)"
        );
        let result = self
            .call_fn1("build_headers", self.config_dynamic())
            .await?;
        let headers = dynamic_to_header_map(self.config_name(), result)?;
        tracing::warn!(
            provider = %self.config_name(),
            header_count = headers.len(),
            header_names = ?headers.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
            "[rhai-dispatch] invoke_build_headers: Rhai returned headers"
        );
        Ok(headers)
    }

    /// Invoke `build_request(request)` and return its JSON body.
    ///
    /// `thinking_config` is forwarded into the Rhai `request` map under
    /// the `thinking_config` key (bridged as `()` when `None`). Scripts
    /// that need adaptive-thinking support can read this field; scripts
    /// that don't can ignore it (PROV-090).
    pub async fn invoke_build_request(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        thinking_config: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ProviderError> {
        tracing::warn!(
            provider = %self.config_name(),
            message_count = messages.len(),
            tool_count = tools.len(),
            has_thinking_config = thinking_config.is_some(),
            "[rhai-dispatch] invoke_build_request: calling Rhai build_request"
        );
        let result = self
            .invoke_request_builder("build_request", messages, tools, thinking_config)
            .await?;
        tracing::warn!(
            provider = %self.config_name(),
            body_preview = %truncate_json_preview(&result, 512),
            "[rhai-dispatch] invoke_build_request: Rhai returned request body"
        );
        Ok(result)
    }

    /// Shared helper for `build_request` / `build_stream_request`:
    /// convert the `(messages, tools, thinking_config)` triple to a
    /// Rhai `request` map, invoke the named script function, and
    /// serialise the returned `Dynamic` back to JSON.
    ///
    /// `fn_name` is `"build_request"` from
    /// [`Self::invoke_build_request`] and
    /// `"build_stream_request"` from
    /// [`crate::custom::provider_stream`].
    pub(crate) async fn invoke_request_builder(
        &self,
        fn_name: &'static str,
        messages: &[Message],
        tools: &[ToolDefinition],
        thinking_config: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ProviderError> {
        let request = request_to_rhai(messages, tools, thinking_config.as_ref())
            .map_err(ProviderError::from)?;
        let result = self.call_fn1(fn_name, request).await?;
        Ok(dynamic_to_json_value(&result))
    }

    /// Invoke `parse_response(raw)` and bridge it to `CompletionResponse`.
    ///
    /// The usage snapshot alongside the response is discarded here
    /// because the `LlmProvider::complete_with_tools` signature cannot
    /// carry it. Callers that need usage (the rig CompletionModel
    /// bridge) should use [`Self::invoke_parse_response_with_usage`]
    /// instead.
    pub async fn invoke_parse_response(
        &self,
        body: &serde_json::Value,
    ) -> Result<CompletionResponse, ProviderError> {
        let (response, _usage) = self.invoke_parse_response_with_usage(body).await?;
        Ok(response)
    }

    /// PROV-103: Same as [`Self::invoke_parse_response`] but also
    /// returns the optional token-usage snapshot surfaced by the
    /// script's `parse_response` map.
    pub async fn invoke_parse_response_with_usage(
        &self,
        body: &serde_json::Value,
    ) -> Result<(CompletionResponse, StreamUsage), ProviderError> {
        tracing::warn!(
            provider = %self.config_name(),
            body_preview = %truncate_json_preview(body, 512),
            "[rhai-dispatch] invoke_parse_response: calling Rhai parse_response"
        );
        let raw = json_value_to_dynamic(body);
        let result = self.call_fn1("parse_response", raw).await?;
        let (response, usage) =
            rhai_to_completion_response(result).map_err(ProviderError::from)?;
        tracing::warn!(
            provider = %self.config_name(),
            stop_reason = ?response.stop_reason,
            input_tokens = ?usage.input_tokens,
            output_tokens = ?usage.output_tokens,
            "[rhai-dispatch] invoke_parse_response: Rhai returned completion"
        );
        Ok((response, usage))
    }

    /// Invoke `map_error(status, body)` and translate the returned map
    /// into a `ProviderError`. Falls back to HTTP status-code
    /// heuristics when the script returns something unexpected.
    pub async fn invoke_map_error(&self, status: u16, body: &str) -> ProviderError {
        tracing::warn!(
            provider = %self.config_name(),
            status = status,
            body_preview = %truncate_str(body, 512),
            "[rhai-dispatch] invoke_map_error: calling Rhai map_error"
        );
        let status_dyn = Dynamic::from(status as i64);
        let body_dyn = Dynamic::from(body.to_string());
        match self.call_fn2("map_error", status_dyn, body_dyn).await {
            Ok(result) => dynamic_to_provider_error(self.config_name(), status, body, result),
            Err(e) => e,
        }
    }
}
