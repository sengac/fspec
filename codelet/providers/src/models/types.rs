//! Model types for models.dev API data structures
//!
//! These types represent the data from https://models.dev/api.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Response from models.dev API containing all providers and their models
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsDevResponse {
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderInfo>,
}

/// Information about a provider from models.dev
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderInfo {
    /// Provider identifier (e.g., "anthropic", "google", "openai")
    pub id: String,
    /// Display name (e.g., "Anthropic", "Google", "OpenAI")
    pub name: String,
    /// Environment variable names for API keys
    #[serde(default)]
    pub env: Vec<String>,
    /// NPM package name for AI SDK
    pub npm: Option<String>,
    /// Base API URL (for OpenAI-compatible providers)
    pub api: Option<String>,
    /// Documentation URL
    pub doc: Option<String>,
    /// Models offered by this provider
    #[serde(default)]
    pub models: HashMap<String, ModelInfo>,
}

/// Information about a specific model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    /// Model identifier (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (e.g., "Claude Sonnet 4")
    pub name: String,
    /// Model family (e.g., "claude-sonnet")
    pub family: Option<String>,
    /// Release date (e.g., "2025-05-14")
    #[serde(default)]
    pub release_date: Option<String>,

    // Capabilities
    /// Whether the model supports file/image attachments
    #[serde(default)]
    pub attachment: bool,
    /// Whether the model supports extended thinking/reasoning
    #[serde(default)]
    pub reasoning: bool,
    /// Whether the model supports function/tool calling
    #[serde(default)]
    pub tool_call: bool,
    /// Whether the model supports temperature parameter
    #[serde(default)]
    pub temperature: bool,

    /// Interleaved thinking configuration
    #[serde(default)]
    pub interleaved: Option<InterleavedConfig>,

    /// Input/output modalities
    pub modalities: Option<Modalities>,

    /// Cost per million tokens
    pub cost: Option<CostInfo>,

    /// Token limits
    pub limit: LimitInfo,

    /// Model status
    pub status: Option<ModelStatus>,
    /// Whether the model is experimental
    #[serde(default)]
    pub experimental: Option<bool>,

    /// Provider-specific options
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    /// Provider-specific headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Interleaved thinking configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InterleavedConfig {
    /// Simple boolean flag
    Boolean(bool),
    /// Configuration with field name
    Config { field: String },
}

/// Input/output modalities for a model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Modalities {
    /// Supported input modalities
    #[serde(default)]
    pub input: Vec<Modality>,
    /// Supported output modalities
    #[serde(default)]
    pub output: Vec<Modality>,
}

/// Media modality types
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Audio,
    Image,
    Video,
    Pdf,
}

/// Cost information for a model (per million tokens)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CostInfo {
    /// Input cost per million tokens
    pub input: f64,
    /// Output cost per million tokens
    pub output: f64,
    /// Cache read cost per million tokens
    pub cache_read: Option<f64>,
    /// Cache write cost per million tokens
    pub cache_write: Option<f64>,
    /// Special pricing for context over 200k tokens
    pub context_over_200k: Option<Box<CostInfo>>,
}

/// Token limits for a model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitInfo {
    /// Maximum context window size in tokens
    pub context: u32,
    /// Maximum output tokens
    pub output: u32,
}

impl Default for LimitInfo {
    fn default() -> Self {
        Self {
            context: 128000,
            output: 4096,
        }
    }
}

/// Model status
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
}

/// Capability filter for model queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Models that support reasoning/extended thinking
    Reasoning,
    /// Models that support image input
    Vision,
    /// Models that support tool/function calling
    ToolCall,
    /// Models that support attachments
    Attachment,
}

impl ModelInfo {
    /// Check if model has a specific capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        match cap {
            Capability::Reasoning => self.reasoning,
            Capability::ToolCall => self.tool_call,
            Capability::Attachment => self.attachment,
            Capability::Vision => self
                .modalities
                .as_ref()
                .map(|m| m.input.contains(&Modality::Image))
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_models_dev_response() {
        let json = r#"{
            "anthropic": {
                "id": "anthropic",
                "name": "Anthropic",
                "env": ["ANTHROPIC_API_KEY"],
                "models": {
                    "claude-sonnet-4": {
                        "id": "claude-sonnet-4-20250514",
                        "name": "Claude Sonnet 4",
                        "reasoning": true,
                        "tool_call": true,
                        "attachment": true,
                        "temperature": true,
                        "limit": {"context": 200000, "output": 16000}
                    }
                }
            }
        }"#;

        let response: ModelsDevResponse = serde_json::from_str(json).unwrap();
        assert!(response.providers.contains_key("anthropic"));

        let anthropic = &response.providers["anthropic"];
        assert_eq!(anthropic.name, "Anthropic");
        assert!(anthropic.models.contains_key("claude-sonnet-4"));

        let claude = &anthropic.models["claude-sonnet-4"];
        assert_eq!(claude.id, "claude-sonnet-4-20250514");
        assert!(claude.reasoning);
        assert!(claude.tool_call);
    }

    #[test]
    fn test_model_capabilities() {
        let model = ModelInfo {
            id: "test-model".to_string(),
            name: "Test Model".to_string(),
            family: None,
            release_date: None,
            attachment: true,
            reasoning: true,
            tool_call: true,
            temperature: true,
            interleaved: None,
            modalities: Some(Modalities {
                input: vec![Modality::Text, Modality::Image],
                output: vec![Modality::Text],
            }),
            cost: None,
            limit: LimitInfo::default(),
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        };

        assert!(model.has_capability(Capability::Reasoning));
        assert!(model.has_capability(Capability::ToolCall));
        assert!(model.has_capability(Capability::Vision));
        assert!(model.has_capability(Capability::Attachment));
    }
}
