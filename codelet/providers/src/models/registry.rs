//! ModelRegistry - Lookup, filtering, and validation for models
//!
//! Provides:
//! - Model string parsing (provider/model-id format)
//! - Model lookup with fuzzy suggestions
//! - Capability filtering
//! - Provider listing

use std::collections::HashMap;

use super::cache::ModelCache;
use super::types::{Capability, ModelInfo, ModelsDevResponse, ProviderInfo};
use crate::error::ProviderError;

/// Registry for model lookup and filtering
pub struct ModelRegistry {
    providers: HashMap<String, ProviderInfo>,
    capability_index: CapabilityIndex,
}

/// Index of models by capability for fast filtering
struct CapabilityIndex {
    reasoning: Vec<(String, String)>,
    vision: Vec<(String, String)>,
    tool_call: Vec<(String, String)>,
    attachment: Vec<(String, String)>,
}

impl Default for CapabilityIndex {
    fn default() -> Self {
        Self {
            reasoning: Vec::new(),
            vision: Vec::new(),
            tool_call: Vec::new(),
            attachment: Vec::new(),
        }
    }
}

impl ModelRegistry {
    /// Create a new registry from cache
    pub async fn new(cache: &ModelCache) -> Result<Self, ProviderError> {
        let data = cache.get().await?;
        Ok(Self::from_response(data))
    }

    /// Create a registry from a models.dev response
    pub fn from_response(response: ModelsDevResponse) -> Self {
        let mut registry = Self {
            providers: response.providers,
            capability_index: CapabilityIndex::default(),
        };
        registry.build_capability_index();
        registry
    }

    /// Parse a model string into (provider, model) tuple
    ///
    /// Supports format: "provider/model-id"
    /// No aliases - full paths only
    pub fn parse_model_string(&self, input: &str) -> Result<(String, String), ProviderError> {
        // Parse provider/model format
        if let Some((provider, model)) = input.split_once('/') {
            // Validate provider exists
            if !self.providers.contains_key(provider) {
                return Err(ProviderError::config(
                    "registry",
                    format!(
                        "Unknown provider: '{}'. Available providers: {}",
                        provider,
                        self.list_provider_ids().join(", ")
                    ),
                ));
            }
            return Ok((provider.to_string(), model.to_string()));
        }

        // No slash found - invalid format
        Err(ProviderError::config(
            "registry",
            format!(
                "Invalid model format: '{}'. Use 'provider/model-id' format (e.g., anthropic/claude-sonnet-4)",
                input
            ),
        ))
    }

    /// Get a model by provider and model ID
    pub fn get_model(&self, provider: &str, model: &str) -> Result<&ModelInfo, ProviderError> {
        let provider_info = self.providers.get(provider).ok_or_else(|| {
            ProviderError::config("registry", format!("Unknown provider: {}", provider))
        })?;

        // Try exact match first
        if let Some(model_info) = provider_info.models.get(model) {
            return Ok(model_info);
        }

        // Try to find a model whose ID starts with the given input
        for (key, model_info) in &provider_info.models {
            if key.starts_with(model) || model_info.id.starts_with(model) {
                return Ok(model_info);
            }
        }

        // Not found - provide suggestions
        let suggestions = self.suggest_models(provider, model);
        Err(ProviderError::config(
            "registry",
            format!(
                "Model '{}' not found in provider '{}'. Did you mean: {}?",
                model,
                provider,
                suggestions.join(", ")
            ),
        ))
    }

    /// Validate that a model can be used (has tool_call capability)
    pub fn validate_model_for_use(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<&ModelInfo, ProviderError> {
        let model_info = self.get_model(provider, model)?;

        if !model_info.tool_call {
            return Err(ProviderError::config(
                "registry",
                format!(
                    "Model {}/{} does not support tool_call. Codelet requires tool_call capability.",
                    provider, model
                ),
            ));
        }

        Ok(model_info)
    }

    /// List all providers
    pub fn list_providers(&self) -> Vec<&ProviderInfo> {
        self.providers.values().collect()
    }

    /// List provider IDs
    pub fn list_provider_ids(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// List models for a specific provider
    pub fn list_models(&self, provider: &str) -> Result<Vec<&ModelInfo>, ProviderError> {
        let provider_info = self.providers.get(provider).ok_or_else(|| {
            ProviderError::config("registry", format!("Unknown provider: {}", provider))
        })?;

        Ok(provider_info.models.values().collect())
    }

    /// Filter models by capability
    pub fn filter_by_capability(&self, cap: Capability) -> Vec<(&str, &ModelInfo)> {
        let keys = match cap {
            Capability::Reasoning => &self.capability_index.reasoning,
            Capability::Vision => &self.capability_index.vision,
            Capability::ToolCall => &self.capability_index.tool_call,
            Capability::Attachment => &self.capability_index.attachment,
        };

        keys.iter()
            .filter_map(|(p, m)| {
                self.providers
                    .get(p)
                    .and_then(|pi| pi.models.get(m))
                    .map(|mi| (p.as_str(), mi))
            })
            .collect()
    }

    /// Search for models by name or ID
    pub fn search(&self, query: &str) -> Vec<(&str, &ModelInfo)> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (provider_id, provider) in &self.providers {
            for (_model_key, model) in &provider.models {
                if model.id.to_lowercase().contains(&query_lower)
                    || model.name.to_lowercase().contains(&query_lower)
                {
                    results.push((provider_id.as_str(), model));
                }
            }
        }

        results
    }

    /// Suggest similar model names for fuzzy matching
    fn suggest_models(&self, provider: &str, input: &str) -> Vec<String> {
        let Some(provider_info) = self.providers.get(provider) else {
            return Vec::new();
        };

        let input_lower = input.to_lowercase();
        let mut suggestions: Vec<(String, usize)> = provider_info
            .models
            .keys()
            .map(|key| {
                let key_lower = key.to_lowercase();
                let distance = levenshtein_distance(&input_lower, &key_lower);
                (key.clone(), distance)
            })
            .collect();

        // Sort by edit distance
        suggestions.sort_by_key(|(_, d)| *d);

        // Return top 3
        suggestions
            .into_iter()
            .take(3)
            .map(|(k, _)| k)
            .collect()
    }

    /// Build the capability index for fast filtering
    fn build_capability_index(&mut self) {
        for (provider_id, provider) in &self.providers {
            for (model_key, model) in &provider.models {
                if model.reasoning {
                    self.capability_index
                        .reasoning
                        .push((provider_id.clone(), model_key.clone()));
                }
                if model.tool_call {
                    self.capability_index
                        .tool_call
                        .push((provider_id.clone(), model_key.clone()));
                }
                if model.attachment {
                    self.capability_index
                        .attachment
                        .push((provider_id.clone(), model_key.clone()));
                }
                if model.has_capability(Capability::Vision) {
                    self.capability_index
                        .vision
                        .push((provider_id.clone(), model_key.clone()));
                }
            }
        }
    }
}

/// Simple Levenshtein distance for fuzzy matching
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::types::{LimitInfo, Modalities, Modality};

    fn create_test_response() -> ModelsDevResponse {
        let mut providers = HashMap::new();

        // Anthropic provider
        let mut anthropic_models = HashMap::new();
        anthropic_models.insert(
            "claude-sonnet-4".to_string(),
            ModelInfo {
                id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                family: Some("claude-sonnet".to_string()),
                release_date: Some("2025-05-14".to_string()),
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
                limit: LimitInfo {
                    context: 200000,
                    output: 16000,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );
        anthropic_models.insert(
            "claude-opus-4".to_string(),
            ModelInfo {
                id: "claude-opus-4-5-20251101".to_string(),
                name: "Claude Opus 4".to_string(),
                family: Some("claude-opus".to_string()),
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
            },
        );

        providers.insert(
            "anthropic".to_string(),
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                env: vec!["ANTHROPIC_API_KEY".to_string()],
                npm: Some("@ai-sdk/anthropic".to_string()),
                api: None,
                doc: None,
                models: anthropic_models,
            },
        );

        ModelsDevResponse { providers }
    }

    #[test]
    fn test_parse_model_string() {
        let registry = ModelRegistry::from_response(create_test_response());

        let result = registry.parse_model_string("anthropic/claude-sonnet-4");
        assert!(result.is_ok());
        let (provider, model) = result.unwrap();
        assert_eq!(provider, "anthropic");
        assert_eq!(model, "claude-sonnet-4");
    }

    #[test]
    fn test_parse_model_string_invalid_format() {
        let registry = ModelRegistry::from_response(create_test_response());

        let result = registry.parse_model_string("claude-sonnet-4");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_model() {
        let registry = ModelRegistry::from_response(create_test_response());

        let model = registry.get_model("anthropic", "claude-sonnet-4");
        assert!(model.is_ok());
        assert_eq!(model.unwrap().name, "Claude Sonnet 4");
    }

    #[test]
    fn test_get_model_with_suggestions() {
        let registry = ModelRegistry::from_response(create_test_response());

        // Use "sonnt" instead of "claud" because "claud" matches via prefix matching
        let result = registry.get_model("anthropic", "sonnt");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Did you mean"));
    }

    #[test]
    fn test_list_providers() {
        let registry = ModelRegistry::from_response(create_test_response());

        let providers = registry.list_providers();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "Anthropic");
    }

    #[test]
    fn test_filter_by_capability() {
        let registry = ModelRegistry::from_response(create_test_response());

        let reasoning_models = registry.filter_by_capability(Capability::Reasoning);
        assert!(!reasoning_models.is_empty());

        let vision_models = registry.filter_by_capability(Capability::Vision);
        assert!(!vision_models.is_empty());
    }

    #[test]
    fn test_search() {
        let registry = ModelRegistry::from_response(create_test_response());

        let results = registry.search("claude");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("claude", "claud"), 1);
        assert_eq!(levenshtein_distance("sonnet", "sonnet"), 0);
        assert_eq!(levenshtein_distance("gpt", "gemini"), 5);
    }
}
