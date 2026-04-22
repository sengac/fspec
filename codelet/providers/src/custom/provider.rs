//! `RhaiCustomProvider` — the core `LlmProvider` implementation backed by
//! a Rhai script (PROV-063).
//!
//! All Rhai calls run on `tokio::task::spawn_blocking`. HTTP uses an
//! async `reqwest::Client`. Helpers live in sibling modules
//! (`request_bridge`, `response_bridge`, `error_mapping`, `rhai_call`,
//! `http`, `model_limits`, `provider_dispatch`) so this file stays
//! focused on the struct definition, construction, accessors, and the
//! `LlmProvider` trait implementation.

use std::sync::Arc;

use async_trait::async_trait;
use codelet_common::Message;
use codelet_tools::ToolDefinition;
use rhai::{Dynamic, Engine, Map, AST};

use super::error::CustomProviderError;
use super::http::post_json;
use super::model_limits::invoke_get_model_limits;
use super::{ProviderConfig, ScriptLoader};
use crate::error::ProviderError;
use crate::{extract_text_from_content, CompletionResponse, LlmProvider};

/// Build the Rhai `config` map passed to user-script lifecycle hooks
/// (`build_url`, `build_headers`, `get_model_limits`, etc.). Shared
/// helper so `RhaiCustomProvider::new` can construct the map BEFORE
/// `self` exists (needed for the PROV-095 `get_model_limits` invocation
/// that happens inside `new`).
fn build_config_dynamic(config: &ProviderConfig, model_id: &str, model_alias: &str) -> Dynamic {
    let mut map = Map::new();
    map.insert("name".into(), Dynamic::from(config.name.clone()));
    map.insert("base_url".into(), Dynamic::from(config.base_url.clone()));
    map.insert("model".into(), Dynamic::from(model_id.to_string()));
    map.insert(
        "model_alias".into(),
        Dynamic::from(model_alias.to_string()),
    );
    Dynamic::from_map(map)
}

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
    /// Context window of the selected model. PROV-095: overridden by
    /// `get_model_limits(config).context_window` when the script defines
    /// that optional hook and returns a positive integer.
    context_window: usize,
    /// Max output tokens of the selected model. PROV-095: overridden by
    /// `get_model_limits(config).max_output_tokens` when the script
    /// defines that optional hook and returns a positive integer.
    max_output_tokens: usize,
    /// PROV-095: `true` when `get_model_limits` supplied a valid
    /// positive `context_window`, `false` when it fell back to the JSON
    /// ModelDef value. Enables the NAPI bridge to tell "script said
    /// 128_000" (authoritative) from "script didn't say anything and
    /// ModelDef defaulted to 128_000" (overridable by TUI value).
    context_window_from_script: bool,
    /// PROV-095: `true` when `get_model_limits` supplied a valid
    /// positive `max_output_tokens`, `false` when it fell back to the
    /// JSON ModelDef value.
    max_output_tokens_from_script: bool,
    /// PROV-095: Compaction-threshold override surfaced by
    /// `get_model_limits(config).compaction_threshold`. `None` when the
    /// script did not define the hook or returned an invalid shape. The
    /// NAPI session-creation path reads this via
    /// [`Self::script_compaction_threshold`] and forwards it into
    /// [`crate::ProviderManager::set_compaction_threshold_override`].
    script_compaction_threshold: Option<(String, u64)>,
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
        let json_context_window = model_def.context_window;
        let json_max_output_tokens = model_def.max_output_tokens;

        let script_path = std::path::PathBuf::from(&config.script);
        let ast = loader.load(&script_path)?;
        loader.validate_required_functions(&ast)?;
        let engine: Arc<Engine> = loader.engine_arc();

        // PROV-095: Give the script a chance to override per-model
        // limits. The optional `get_model_limits(config)` hook is
        // invoked once here and its result is cached in the struct
        // fields for the provider's lifetime (rule 8). Missing hook
        // and/or invalid return values are silently ignored in favor
        // of the JSON ModelDef values (rules 4/6).
        let config_map = build_config_dynamic(&config, &model_id, &model_alias);
        let overrides = invoke_get_model_limits(&engine, &ast, &config.name, config_map);
        let context_window_from_script = overrides.context_window.is_some();
        let max_output_tokens_from_script = overrides.max_output_tokens.is_some();
        let context_window = overrides.context_window.unwrap_or(json_context_window);
        let max_output_tokens = overrides
            .max_output_tokens
            .unwrap_or(json_max_output_tokens);
        let script_compaction_threshold = overrides.compaction_threshold;

        Ok(Self {
            config,
            model_alias,
            model_id,
            context_window,
            max_output_tokens,
            context_window_from_script,
            max_output_tokens_from_script,
            script_compaction_threshold,
            ast,
            engine,
            _loader: loader,
            http_client: reqwest::Client::new(),
        })
    }

    /// Build the `config` map passed to script functions.
    pub(crate) fn config_dynamic(&self) -> Dynamic {
        build_config_dynamic(&self.config, &self.model_id, &self.model_alias)
    }

    /// PROV-095: Accessor exposing the compaction-threshold override
    /// surfaced by the optional `get_model_limits(config).compaction_threshold`
    /// Rhai hook. Returns `None` when the script did not define the
    /// hook, did not include a `compaction_threshold` entry, or returned
    /// an invalid shape (non-"tokens"/"percentage" kind, non-positive
    /// tokens value, or percentage outside 1..=100).
    ///
    /// The NAPI session-creation path consults this accessor after
    /// constructing a `RhaiCustomProvider` and forwards the tuple into
    /// [`crate::ProviderManager::set_compaction_threshold_override`].
    pub fn script_compaction_threshold(&self) -> Option<(String, u64)> {
        self.script_compaction_threshold.clone()
    }

    /// PROV-095: Accessor exposing the script-supplied `context_window`.
    /// Returns `Some(n)` only when `get_model_limits` returned a valid
    /// positive integer for `context_window`; returns `None` when the
    /// script did not define the hook or its value was rejected.
    ///
    /// Used by the NAPI bridge to decide whether to override the
    /// TUI-supplied (or default) context window when setting up a
    /// custom-provider session.
    pub fn script_context_window(&self) -> Option<usize> {
        if self.context_window_from_script {
            Some(self.context_window)
        } else {
            None
        }
    }

    /// PROV-095: Accessor exposing the script-supplied `max_output_tokens`.
    /// Returns `Some(n)` only when `get_model_limits` returned a valid
    /// positive integer for `max_output_tokens`; returns `None` otherwise.
    pub fn script_max_output_tokens(&self) -> Option<usize> {
        if self.max_output_tokens_from_script {
            Some(self.max_output_tokens)
        } else {
            None
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

    /// Accessor that re-exports `config_dynamic` for sibling modules
    /// (`provider_stream`, `rig_model`) that need the typed map without
    /// widening the pub surface of the primary type.
    pub(crate) fn config_dynamic_accessor(&self) -> Dynamic {
        self.config_dynamic()
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
        let body = self.invoke_build_request(messages, tools, None).await?;

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
