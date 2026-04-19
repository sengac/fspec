//! `RhaiCustomProvider` — the core `LlmProvider` implementation backed by
//! a Rhai script (PROV-063).
//!
//! All Rhai calls run on `tokio::task::spawn_blocking`. HTTP uses an
//! async `reqwest::Client`. Helpers live in sibling modules
//! (`request_bridge`, `response_bridge`, `error_mapping`, `rhai_call`,
//! `http`) to keep this file under the 300-line threshold.

use std::sync::Arc;

use async_trait::async_trait;
use codelet_common::Message;
use codelet_tools::ToolDefinition;
use reqwest::header::HeaderMap;
use rhai::{Dynamic, Engine, Map, AST};

use super::conversion::{dynamic_to_json_value, json_value_to_dynamic};
use super::error::CustomProviderError;
use super::error_mapping::dynamic_to_provider_error;
use super::http::{dynamic_to_header_map, post_json};
use super::request_bridge::request_to_rhai;
use super::response_bridge::rhai_to_completion_response;
use super::rhai_call::{call_fn1, call_fn2};
use super::{ProviderConfig, ScriptLoader};
use crate::error::ProviderError;
use crate::{extract_text_from_content, CompletionResponse, LlmProvider};

/// Custom LLM provider backed by a Rhai script. `Arc`-heavy so it's
/// cheap to clone into `ProviderManager`.
#[derive(Clone)]
pub struct RhaiCustomProvider {
    /// Provider configuration loaded from JSON.
    config: Arc<ProviderConfig>,
    /// Model alias (a key of `config.models`).
    model_alias: String,
    /// Resolved `models[model_alias].id` (sent to the LLM API).
    model_id: String,
    /// Context window of the selected model.
    context_window: usize,
    /// Max output tokens of the selected model.
    max_output_tokens: usize,
    /// Compiled Rhai AST.
    ast: Arc<AST>,
    /// Rhai engine (shared from the loader).
    engine: Arc<Engine>,
    /// Loader kept alive so its cache remains valid across reloads.
    _loader: Arc<ScriptLoader>,
    /// Shared HTTP client for outgoing requests.
    http_client: reqwest::Client,
}

impl RhaiCustomProvider {
    /// Construct a new provider from a config + a script loader + the
    /// alias of the model to use (must be a key of `config.models`).
    pub fn new(
        config: Arc<ProviderConfig>,
        loader: Arc<ScriptLoader>,
        model_alias: String,
    ) -> Result<Self, CustomProviderError> {
        let model_def = config.models.get(&model_alias).ok_or_else(|| {
            CustomProviderError::RhaiRuntimeError(format!(
                "model alias '{model_alias}' not found in config.models"
            ))
        })?;
        let model_id = model_def.id.clone();
        let context_window = model_def.context_window;
        let max_output_tokens = model_def.max_output_tokens;

        let script_path = std::path::PathBuf::from(&config.script);
        let ast = loader.load(&script_path)?;
        loader.validate_required_functions(&ast)?;
        let engine: Arc<Engine> = loader.engine_arc();

        Ok(Self {
            config,
            model_alias,
            model_id,
            context_window,
            max_output_tokens,
            ast,
            engine,
            _loader: loader,
            http_client: reqwest::Client::new(),
        })
    }

    /// Build the `config` map passed to script functions.
    pub(crate) fn config_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from(self.config.name.clone()));
        map.insert(
            "base_url".into(),
            Dynamic::from(self.config.base_url.clone()),
        );
        map.insert("model".into(), Dynamic::from(self.model_id.clone()));
        map.insert(
            "model_alias".into(),
            Dynamic::from(self.model_alias.clone()),
        );
        Dynamic::from_map(map)
    }

    /// Offload a 1-arg Rhai call to `spawn_blocking`.
    pub(crate) async fn call_fn1(
        &self,
        fn_name: &'static str,
        arg: Dynamic,
    ) -> Result<Dynamic, ProviderError> {
        call_fn1(
            self.engine.clone(),
            self.ast.clone(),
            self.config.name.clone(),
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
            self.engine.clone(),
            self.ast.clone(),
            self.config.name.clone(),
            fn_name,
            arg1,
            arg2,
        )
        .await
    }

    /// Invoke `build_url(config)` and return the resulting URL string.
    pub async fn invoke_build_url(&self) -> Result<String, ProviderError> {
        let result = self.call_fn1("build_url", self.config_dynamic()).await?;
        result.into_string().map_err(|typ| {
            ProviderError::api(
                self.config.name.clone(),
                format!("build_url must return a string, got {typ}"),
            )
        })
    }

    /// Invoke `build_headers(config)` and return a `HeaderMap`.
    pub async fn invoke_build_headers(&self) -> Result<HeaderMap, ProviderError> {
        let result = self
            .call_fn1("build_headers", self.config_dynamic())
            .await?;
        dynamic_to_header_map(&self.config.name, result)
    }

    /// Invoke `build_request(request)` and return its JSON body.
    pub async fn invoke_build_request(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<serde_json::Value, ProviderError> {
        let request = request_to_rhai(messages, tools).map_err(ProviderError::from)?;
        let result = self.call_fn1("build_request", request).await?;
        Ok(dynamic_to_json_value(&result))
    }

    /// Invoke `parse_response(raw)` and bridge it to `CompletionResponse`.
    pub async fn invoke_parse_response(
        &self,
        body: &serde_json::Value,
    ) -> Result<CompletionResponse, ProviderError> {
        let raw = json_value_to_dynamic(body);
        let result = self.call_fn1("parse_response", raw).await?;
        rhai_to_completion_response(result).map_err(ProviderError::from)
    }

    /// Invoke `map_error(status, body)` and translate the returned map
    /// into a `ProviderError`. Falls back to HTTP status-code
    /// heuristics when the script returns something unexpected.
    pub async fn invoke_map_error(&self, status: u16, body: &str) -> ProviderError {
        let status_dyn = Dynamic::from(status as i64);
        let body_dyn = Dynamic::from(body.to_string());
        match self.call_fn2("map_error", status_dyn, body_dyn).await {
            Ok(result) => dynamic_to_provider_error(&self.config.name, status, body, result),
            Err(e) => e,
        }
    }

    /// Accessor for the provider name (used by streaming glue).
    pub(crate) fn config_name(&self) -> &str {
        &self.config.name
    }

    /// Accessor for the shared Rhai engine.
    pub(crate) fn engine_handle(&self) -> std::sync::Arc<rhai::Engine> {
        self.engine.clone()
    }

    /// Accessor for the compiled AST.
    pub(crate) fn ast_handle(&self) -> std::sync::Arc<rhai::AST> {
        self.ast.clone()
    }

    /// Accessor for the shared async HTTP client.
    pub(crate) fn http_client_handle(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Accessor that re-exports `config_dynamic` for the streaming module.
    pub(crate) fn config_dynamic_accessor(&self) -> Dynamic {
        self.config_dynamic()
    }

    /// Accessor that re-exports the 1-arg Rhai caller for the streaming module.
    pub(crate) async fn call_fn1_accessor(
        &self,
        fn_name: &'static str,
        arg: Dynamic,
    ) -> Result<Dynamic, ProviderError> {
        self.call_fn1(fn_name, arg).await
    }
}

#[async_trait]
impl LlmProvider for RhaiCustomProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn model(&self) -> &str {
        &self.model_id
    }

    fn context_window(&self) -> usize {
        self.context_window
    }

    fn max_output_tokens(&self) -> usize {
        self.max_output_tokens
    }

    fn supports_caching(&self) -> bool {
        false
    }

    fn supports_streaming(&self) -> bool {
        self.config
            .models
            .get(&self.model_alias)
            .map(|m| m.supports_streaming)
            .unwrap_or(false)
    }

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError> {
        let response = self.complete_with_tools(messages, &[]).await?;
        Ok(extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        let url = self.invoke_build_url().await?;
        let headers = self.invoke_build_headers().await?;
        let body = self.invoke_build_request(messages, tools).await?;

        let (status_code, body_text) =
            post_json(&self.http_client, &self.config.name, &url, headers, &body).await?;

        if (200..300).contains(&status_code) {
            let body_json: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                ProviderError::api(
                    self.config.name.clone(),
                    format!("response body was not valid JSON: {e}"),
                )
            })?;
            self.invoke_parse_response(&body_json).await
        } else {
            tracing::debug!(status = status_code, "custom provider non-success response");
            Err(self.invoke_map_error(status_code, &body_text).await)
        }
    }
}
