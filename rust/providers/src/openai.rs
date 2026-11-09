//! OpenAI Provider implementation using rig
//!
//! Implements the LlmProvider trait for OpenAI API communication.
//! Uses rig::providers::openai for HTTP communication.
//!
//! PROV-006: Supports custom base URLs for local model servers (vLLM, Ollama, LM Studio)
//! via OPENAI_BASE_URL environment variable.

use crate::ProviderAdapter;
use rig::providers::openai;

#[path = "openai_discovery.rs"]
mod discovery;

mod agent;
mod construction;
mod env_limits;
mod llm_provider;
#[cfg(test)]
mod tests;

pub use env_limits::OpenAIEnvLimits;

/// Anti-ramble sampling settings applied to LOCAL endpoints only
/// (i.e. when `OPENAI_BASE_URL` is set — Ollama, vLLM, LM Studio, LocalAI,
/// llama.cpp, or any other OpenAI-compatible local server). Cloud OpenAI
/// requests are left untouched.
///
/// | Setting            | Value |
/// | ------------------ | ----- |
/// | Temperature        | 0.7   |
/// | Top P              | 0.80  |
/// | Top K              | 20    |
/// | Min P              | 0.0   |
/// | Presence penalty   | 1.5   |
/// | Repetition penalty | 1.0   |
///
/// `top_k` / `min_p` are sent as flat OpenAI-compatible fields. Servers that
/// ignore unknown fields (OpenAI cloud, vLLM defaults) simply drop them, so
/// the same body is safe on every local server that accepts OpenAI chat
/// requests.
const ANTI_RAMBLE_TEMPERATURE: f64 = 0.7;
const ANTI_RAMBLE_TOP_P: f64 = 0.8;
const ANTI_RAMBLE_TOP_K: u32 = 20;
const ANTI_RAMBLE_MIN_P: f64 = 0.0;
const ANTI_RAMBLE_PRESENCE_PENALTY: f64 = 1.5;
const ANTI_RAMBLE_REPETITION_PENALTY: f64 = 1.0;

/// Build the anti-ramble sampling body for local endpoints.
fn anti_ramble_additional_params() -> serde_json::Value {
    serde_json::json!({
        "top_p": ANTI_RAMBLE_TOP_P,
        "top_k": ANTI_RAMBLE_TOP_K,
        "min_p": ANTI_RAMBLE_MIN_P,
        "presence_penalty": ANTI_RAMBLE_PRESENCE_PENALTY,
        "repetition_penalty": ANTI_RAMBLE_REPETITION_PENALTY,
    })
}

/// BUG-440: merge the agent loop's thinking parameters (e.g.
/// `{"reasoning_effort": "high"}`) into a provider's additional request
/// parameters. The agent loop only produces JSON objects
/// (`parse_thinking_config` refuses anything else); a non-object here has no
/// error channel, so it is logged as an error and not sent.
pub fn merge_thinking_params(
    params: serde_json::Value,
    thinking_config: Option<serde_json::Value>,
) -> serde_json::Value {
    let Some(thinking) = thinking_config else {
        return params;
    };
    let (serde_json::Value::Object(mut merged), serde_json::Value::Object(thinking)) =
        (params.clone(), thinking.clone())
    else {
        tracing::error!(
            "thinking parameters {thinking} cannot be merged into request parameters {params}; the thinking level is not sent"
        );
        return params;
    };
    merged.extend(thinking);
    serde_json::Value::Object(merged)
}

/// Normalize base URL to ensure it includes /v1 suffix
///
/// The rig library's default OpenAI base URL is "https://api.openai.com/v1",
/// which includes the /v1 prefix. When using custom base URLs for local servers
/// (vLLM, Ollama, etc.), the URL typically doesn't include /v1, but the rig
/// client expects it because it appends paths like /chat/completions directly.
///
/// This function ensures consistency by adding /v1 if not present.
fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

/// OpenAI Provider for OpenAI API (using rig)
///
/// PROV-006: Now supports custom base URLs for local model servers via:
/// - OPENAI_BASE_URL: Custom API endpoint (e.g., http://localhost:8888)
/// - OPENAI_CONTEXT_WINDOW: Custom context window size
/// - OPENAI_MAX_OUTPUT_TOKENS: Custom max output tokens
///
/// PROV-171: an unset variable is no override (the limit is `None`); an
/// invalid one fails construction (see [`OpenAIEnvLimits`]).
#[derive(Clone)]
pub struct OpenAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
    /// Custom base URL if set (None for default OpenAI API)
    base_url: Option<String>,
    /// Context window (OPENAI_CONTEXT_WINDOW); `None` when not declared
    context_window: Option<usize>,
    /// Max output tokens (OPENAI_MAX_OUTPUT_TOKENS or the session's resolved
    /// limit); `None` when not declared
    max_output_tokens: Option<usize>,
    /// PROV-140: per-profile streaming toggle sourced from `OPENAI_STREAMING`
    /// (default `true`; only an explicit "false" disables). Drives
    /// `supports_streaming()` and the rig model's request path.
    streaming: bool,
}

impl std::fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("model", &self.model_name)
            .field("base_url", &self.base_url)
            .field("context_window", &self.context_window)
            .field("max_output_tokens", &self.max_output_tokens)
            .finish()
    }
}

impl ProviderAdapter for OpenAIProvider {
    fn provider_name(&self) -> &'static str {
        "openai"
    }
}
