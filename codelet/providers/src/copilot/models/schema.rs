//! Schema types for the Copilot `/models` endpoint response (PROV-056).
//!
//! Every field here maps directly onto JSON returned by the live Copilot
//! `/models` endpoint. There is intentionally **no** derived field, no
//! computed default, and no per-model branching — this is a pure wire-format
//! mirror.

use serde::{Deserialize, Serialize};

/// JSON shape of the Copilot `/models` response.
///
/// The endpoint returns `{ "data": [ { ... }, { ... } ] }`. Only the fields
/// the catalog needs are deserialized; unknown fields are tolerated because
/// every sub-struct uses `#[serde(default)]` where it matters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModelsResponse {
    /// Models advertised by the endpoint.
    pub data: Vec<CopilotModelEntry>,
}

/// One entry inside `CopilotModelsResponse.data`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModelEntry {
    /// Model identifier (e.g. whatever the endpoint advertises).
    pub id: String,
    /// Display name from the endpoint.
    pub name: String,
    /// Version field used to derive `release_date`.
    pub version: String,
    /// Whether this model should appear in the user-facing picker.
    pub model_picker_enabled: bool,
    /// Capability bag.
    pub capabilities: CopilotModelCapabilities,
}

/// `capabilities` sub-object on a Copilot model entry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModelCapabilities {
    /// Family identifier as advertised by the endpoint.
    pub family: String,
    /// Token limits.
    pub limits: CopilotModelLimits,
    /// Feature support flags.
    pub supports: CopilotModelSupports,
}

/// `capabilities.limits` sub-object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModelLimits {
    /// Maximum context window size in tokens.
    pub max_context_window_tokens: u64,
    /// Maximum output tokens.
    pub max_output_tokens: u64,
    /// Maximum prompt tokens.
    pub max_prompt_tokens: u64,
}

/// `capabilities.supports` sub-object.
///
/// `reasoning_effort` is `Option` because the endpoint may omit the field
/// entirely; missing → `None`, empty array → `Some(vec![])`. Both collapse
/// to an empty `reasoning_variants` list in the resulting `ModelInfo`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CopilotModelSupports {
    /// Server-sent streaming supported.
    #[serde(default)]
    pub streaming: bool,
    /// Tool / function calling supported.
    #[serde(default)]
    pub tool_calls: bool,
    /// Image input supported.
    #[serde(default)]
    pub vision: bool,
    /// Optional reasoning effort tiers, ordered.
    #[serde(default)]
    pub reasoning_effort: Option<Vec<String>>,
}
