# Model Selection Architecture for fspec/codelet

## Overview

This document describes the architecture for dynamic model selection in fspec/codelet, using [models.dev](https://models.dev) as the single source of truth for model metadata.

## Design Principles

1. **No hardcoded model lists** - All model metadata comes from models.dev API
2. **Facades define behavior, not data** - Provider facades handle API format, auth, and provider-specific options
3. **Indefinite cache** - Cache forever, only refetch on failure or explicit refresh
4. **Lazy fetching** - Only fetch from API when cache missing or corrupted
5. **Capability-driven** - Filter/select models based on capabilities (reasoning, vision, tool_call, etc.)

---

## models.dev API

### Endpoint

```
GET https://models.dev/api.json
```

### Response Structure

```typescript
type ModelsDevResponse = Record<ProviderId, Provider>

interface Provider {
  id: string              // "anthropic", "google", "openai"
  name: string            // "Anthropic", "Google", "OpenAI"
  env: string[]           // ["ANTHROPIC_API_KEY"]
  npm?: string            // "@ai-sdk/anthropic"
  api?: string            // Base API URL (for OpenAI-compatible)
  doc?: string            // Documentation URL
  models: Record<ModelId, Model>
}

interface Model {
  id: string              // "claude-sonnet-4-20250514"
  name: string            // "Claude Sonnet 4"
  family?: string         // "claude-sonnet"
  release_date: string    // "2025-05-14"

  // Capabilities
  attachment: boolean     // File/image attachments
  reasoning: boolean      // Extended thinking support
  tool_call: boolean      // Function calling
  temperature: boolean    // Temperature parameter supported
  interleaved?: boolean | { field: "reasoning_content" | "reasoning_details" }

  // Modalities
  modalities?: {
    input: ("text" | "audio" | "image" | "video" | "pdf")[]
    output: ("text" | "audio" | "image" | "video" | "pdf")[]
  }

  // Costs (per million tokens)
  cost?: {
    input: number
    output: number
    cache_read?: number
    cache_write?: number
    context_over_200k?: {
      input: number
      output: number
      cache_read?: number
    }
  }

  // Limits
  limit: {
    context: number       // Context window size
    output: number        // Max output tokens
  }

  // Status
  status?: "alpha" | "beta" | "deprecated"
  experimental?: boolean

  // Provider-specific
  options?: Record<string, any>
  headers?: Record<string, string>
}
```

### Available Providers (as of Dec 2024)

Major providers from models.dev:
- `anthropic` - Claude models
- `google` - Gemini models
- `openai` - GPT/O1 models
- `amazon-bedrock` - AWS Bedrock
- `google-vertex` - Vertex AI
- `azure` - Azure OpenAI
- `openrouter` - OpenRouter proxy
- `deepseek` - DeepSeek models
- `groq` - Groq inference
- `mistral` - Mistral models
- `xai` - Grok models
- Plus 60+ more providers

---

## Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      User/CLI Layer                          │
│  --model sonnet | --model anthropic/claude-sonnet-4 | etc   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ModelRegistry                             │
│  - parse_model_string() → (provider_id, model_id)           │
│  - resolve_alias() → canonical model                         │
│  - get_model() → ModelInfo                                   │
│  - list_providers() → Vec<ProviderInfo>                      │
│  - list_models(provider) → Vec<ModelInfo>                    │
│  - filter_by_capability(cap) → Vec<ModelInfo>               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ModelCache                                │
│  - get() → returns cached data, fetches only if needed       │
│  - refresh() → force fetch from API (explicit only)          │
│  - Cache location: ~/.codelet/cache/models.json             │
│  - Strategy: Indefinite cache, refetch on failure/refresh   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ProviderFacade                            │
│  (Defines HOW to talk to provider, not WHAT models exist)   │
│                                                              │
│  - AnthropicFacade: API format, auth headers, thinking      │
│  - GoogleFacade: API format, thinking budget                 │
│  - OpenAIFacade: API format, responses endpoint              │
│  - OpenAICompatibleFacade: Generic for compatible APIs       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Existing Facade Infrastructure                  │
│  - SystemPromptFacade (provider-specific formatting)         │
│  - ThinkingConfigFacade (model-specific thinking setup)      │
│  - ToolFacade (provider-specific tool schemas)               │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

#### ModelCache (NEW)
- Fetches models.dev API on startup/refresh
- Stores in `~/.codelet/cache/models.json`
- Checks TTL before serving cached data
- Handles network failures gracefully (use stale cache)

#### ModelRegistry (NEW)
- Parses model strings: `"sonnet"` → `("anthropic", "claude-sonnet-4-...")`
- Provides model discovery: list all, filter by capability
- Merges models.dev data with local overrides (optional)
- Validates model exists before use

#### ProviderFacade (NEW - extends existing pattern)
- Maps provider ID to API behavior
- Does NOT store model lists (gets from registry)
- Provides:
  - `api_format()` → "anthropic" | "openai" | "openai-compatible"
  - `auth_headers(api_key)` → Headers
  - `thinking_facade(model)` → ThinkingConfigFacade
  - `system_prompt_facade(model)` → SystemPromptFacade
  - `transform_request(model, request)` → provider-specific request

---

## Data Flow

### Startup Flow

```
1. codelet starts
2. ModelCache.get() called
   └─ If cache exists and parses successfully:
      └─ Use cached data (no network call)
   └─ If cache missing OR parse fails:
      └─ Fetch https://models.dev/api.json
      └─ Write to ~/.codelet/cache/models.json
      └─ If fetch fails: error (no fallback on first run)
3. ModelRegistry.init(cache_data)
   └─ Parse all providers and models
   └─ Build alias map
   └─ Index by capabilities
4. Ready to accept --model argument
```

### Model Selection Flow

```
User: codelet --model sonnet "do something"

1. ModelRegistry.parse_model_string("sonnet")
   └─ Check aliases: "sonnet" → ("anthropic", "claude-sonnet-4-20250514")
   └─ Or parse "provider/model" format directly

2. ModelRegistry.get_model("anthropic", "claude-sonnet-4-20250514")
   └─ Returns ModelInfo with all metadata from models.dev

3. ProviderFacade.for_provider("anthropic")
   └─ Returns AnthropicFacade

4. AnthropicFacade.thinking_facade(model_info)
   └─ If model.reasoning == true: return ClaudeThinkingFacade
   └─ Else: return NoOpThinkingFacade

5. Agent configured with:
   └─ Model ID from ModelInfo
   └─ System prompt from facade
   └─ Thinking config from facade
   └─ Tool schemas from facade
   └─ Context limits from ModelInfo
```

---

## Rust Implementation

### File Structure

```
codelet/providers/src/
├── lib.rs
├── models/
│   ├── mod.rs
│   ├── cache.rs          # ModelCache - fetches/caches models.dev
│   ├── registry.rs       # ModelRegistry - lookup/filter/alias
│   ├── types.rs          # Provider, Model, Capabilities structs
│   └── aliases.rs        # Built-in aliases (sonnet, opus, flash, etc.)
│
├── facade/
│   ├── mod.rs
│   ├── traits.rs         # ProviderFacade trait
│   ├── anthropic.rs      # AnthropicFacade
│   ├── google.rs         # GoogleFacade
│   ├── openai.rs         # OpenAIFacade
│   └── openai_compat.rs  # OpenAICompatibleFacade (generic)
│
└── (existing files)
    ├── manager.rs        # Update to use ModelRegistry
    └── credentials.rs
```

### Core Types

```rust
// models/types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsDevResponse {
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub env: Vec<String>,
    pub npm: Option<String>,
    pub api: Option<String>,
    pub doc: Option<String>,
    pub models: HashMap<String, ModelInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    pub release_date: String,

    // Capabilities
    pub attachment: bool,
    pub reasoning: bool,
    pub tool_call: bool,
    pub temperature: bool,

    #[serde(default)]
    pub interleaved: Option<InterleavedConfig>,

    pub modalities: Option<Modalities>,
    pub cost: Option<CostInfo>,
    pub limit: LimitInfo,

    pub status: Option<ModelStatus>,
    pub experimental: Option<bool>,

    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InterleavedConfig {
    Boolean(bool),
    Config { field: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Modalities {
    pub input: Vec<Modality>,
    pub output: Vec<Modality>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Audio,
    Image,
    Video,
    Pdf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CostInfo {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub context_over_200k: Option<CostInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitInfo {
    pub context: u32,
    pub output: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
}
```

### ModelCache

```rust
// models/cache.rs

use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

pub struct ModelCache {
    cache_path: PathBuf,
}

impl ModelCache {
    pub fn new() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".codelet")
            .join("cache");

        Self {
            cache_path: cache_dir.join("models.json"),
        }
    }

    /// Get models data. Uses cache if valid, fetches only if needed.
    pub async fn get(&self) -> Result<ModelsDevResponse, CacheError> {
        // Try to read and parse cache first
        match self.read_cache().await {
            Ok(data) => Ok(data),
            Err(e) => {
                log::info!("Cache miss or invalid ({}), fetching from API", e);
                self.fetch_and_cache().await
            }
        }
    }

    /// Force refresh from API (user-initiated via --refresh flag)
    pub async fn refresh(&self) -> Result<ModelsDevResponse, CacheError> {
        log::info!("Force refreshing models cache");
        self.fetch_and_cache().await
    }

    async fn fetch_and_cache(&self) -> Result<ModelsDevResponse, CacheError> {
        let response = reqwest::Client::new()
            .get("https://models.dev/api.json")
            .header("User-Agent", "codelet/0.1")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        let data = response.text().await?;

        // Validate JSON before saving
        let parsed: ModelsDevResponse = serde_json::from_str(&data)?;

        // Ensure cache directory exists
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.cache_path, &data).await?;
        log::info!("Models cache updated: {}", self.cache_path.display());

        Ok(parsed)
    }

    async fn read_cache(&self) -> Result<ModelsDevResponse, CacheError> {
        let data = fs::read_to_string(&self.cache_path).await
            .map_err(|e| CacheError::NotFound(e.to_string()))?;

        serde_json::from_str(&data)
            .map_err(|e| CacheError::ParseError(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Cache not found: {0}")]
    NotFound(String),

    #[error("Failed to parse cache: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### ModelRegistry

```rust
// models/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ModelRegistry {
    providers: HashMap<String, ProviderInfo>,
    aliases: HashMap<String, (String, String)>, // alias → (provider, model)
    by_capability: CapabilityIndex,
}

struct CapabilityIndex {
    reasoning: Vec<(String, String)>,
    vision: Vec<(String, String)>,
    tool_call: Vec<(String, String)>,
}

impl ModelRegistry {
    pub async fn new(cache: &ModelCache) -> Result<Self, RegistryError> {
        let data = cache.get().await?;

        let mut registry = Self {
            providers: data.providers,
            aliases: Self::default_aliases(),
            by_capability: CapabilityIndex::default(),
        };

        registry.build_capability_index();
        Ok(registry)
    }

    fn default_aliases() -> HashMap<String, (String, String)> {
        let mut aliases = HashMap::new();

        // Claude aliases
        aliases.insert("sonnet".into(), ("anthropic".into(), "claude-sonnet-4-20250514".into()));
        aliases.insert("opus".into(), ("anthropic".into(), "claude-opus-4-5-20251101".into()));
        aliases.insert("haiku".into(), ("anthropic".into(), "claude-haiku-4-5-20251015".into()));

        // Gemini aliases
        aliases.insert("flash".into(), ("google".into(), "gemini-2.0-flash".into()));
        aliases.insert("pro".into(), ("google".into(), "gemini-2.5-pro".into()));

        // OpenAI aliases
        aliases.insert("gpt4".into(), ("openai".into(), "gpt-4o".into()));
        aliases.insert("o1".into(), ("openai".into(), "o1".into()));

        aliases
    }

    /// Parse "sonnet" or "anthropic/claude-sonnet-4" into (provider, model)
    pub fn parse_model_string(&self, input: &str) -> Result<(String, String), RegistryError> {
        // Check aliases first
        if let Some((provider, model)) = self.aliases.get(input) {
            return Ok((provider.clone(), model.clone()));
        }

        // Parse provider/model format
        if let Some((provider, model)) = input.split_once('/') {
            // Validate provider exists
            if !self.providers.contains_key(provider) {
                return Err(RegistryError::UnknownProvider(provider.to_string()));
            }
            return Ok((provider.to_string(), model.to_string()));
        }

        // Try fuzzy match across all providers
        self.fuzzy_find(input)
    }

    pub fn get_model(&self, provider: &str, model: &str) -> Result<&ModelInfo, RegistryError> {
        let provider_info = self.providers.get(provider)
            .ok_or_else(|| RegistryError::UnknownProvider(provider.to_string()))?;

        provider_info.models.get(model)
            .ok_or_else(|| RegistryError::UnknownModel {
                provider: provider.to_string(),
                model: model.to_string(),
                suggestions: self.suggest_models(provider, model),
            })
    }

    pub fn list_providers(&self) -> Vec<&ProviderInfo> {
        self.providers.values().collect()
    }

    pub fn list_models(&self, provider: &str) -> Result<Vec<&ModelInfo>, RegistryError> {
        let provider_info = self.providers.get(provider)
            .ok_or_else(|| RegistryError::UnknownProvider(provider.to_string()))?;

        Ok(provider_info.models.values().collect())
    }

    pub fn filter_by_capability(&self, cap: Capability) -> Vec<(&str, &ModelInfo)> {
        let keys = match cap {
            Capability::Reasoning => &self.by_capability.reasoning,
            Capability::Vision => &self.by_capability.vision,
            Capability::ToolCall => &self.by_capability.tool_call,
        };

        keys.iter()
            .filter_map(|(p, m)| {
                self.providers.get(p)
                    .and_then(|pi| pi.models.get(m))
                    .map(|mi| (p.as_str(), mi))
            })
            .collect()
    }

    fn fuzzy_find(&self, input: &str) -> Result<(String, String), RegistryError> {
        let input_lower = input.to_lowercase();

        for (provider_id, provider) in &self.providers {
            for (model_id, model) in &provider.models {
                if model_id.to_lowercase().contains(&input_lower)
                   || model.name.to_lowercase().contains(&input_lower) {
                    return Ok((provider_id.clone(), model_id.clone()));
                }
            }
        }

        Err(RegistryError::NoMatch(input.to_string()))
    }

    fn suggest_models(&self, provider: &str, input: &str) -> Vec<String> {
        // Use edit distance to find similar model names
        // Returns top 3 suggestions
        vec![] // TODO: implement
    }

    fn build_capability_index(&mut self) {
        for (provider_id, provider) in &self.providers {
            for (model_id, model) in &provider.models {
                if model.reasoning {
                    self.by_capability.reasoning.push((provider_id.clone(), model_id.clone()));
                }
                if model.modalities.as_ref()
                    .map(|m| m.input.contains(&Modality::Image))
                    .unwrap_or(false) {
                    self.by_capability.vision.push((provider_id.clone(), model_id.clone()));
                }
                if model.tool_call {
                    self.by_capability.tool_call.push((provider_id.clone(), model_id.clone()));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Capability {
    Reasoning,
    Vision,
    ToolCall,
}
```

### ProviderFacade Trait

```rust
// facade/traits.rs

use async_trait::async_trait;

/// Defines HOW to communicate with a provider.
/// Does NOT define what models exist (that comes from ModelRegistry).
#[async_trait]
pub trait ProviderFacade: Send + Sync {
    /// Provider ID matching models.dev (e.g., "anthropic", "google", "openai")
    fn provider_id(&self) -> &'static str;

    /// API format for request/response transformation
    fn api_format(&self) -> ApiFormat;

    /// Build auth headers from API key
    fn auth_headers(&self, api_key: &str) -> Vec<(String, String)>;

    /// Get thinking config facade for a specific model
    fn thinking_facade(&self, model: &ModelInfo) -> Box<dyn ThinkingConfigFacade>;

    /// Get system prompt facade for a specific model
    fn system_prompt_facade(&self, model: &ModelInfo) -> Box<dyn SystemPromptFacade>;

    /// Transform a generic request into provider-specific format
    fn transform_request(&self, model: &ModelInfo, request: GenericRequest) -> ProviderRequest;

    /// Parse provider-specific response into generic format
    fn parse_response(&self, response: ProviderResponse) -> GenericResponse;

    /// Additional options to include in API calls for this model
    fn model_options(&self, model: &ModelInfo) -> serde_json::Value {
        serde_json::json!({})
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ApiFormat {
    Anthropic,      // Anthropic Messages API
    OpenAI,         // OpenAI Chat/Responses API
    OpenAICompat,   // OpenAI-compatible (generic)
    Google,         // Google GenerativeAI API
}
```

### Anthropic Facade Example

```rust
// facade/anthropic.rs

pub struct AnthropicFacade;

#[async_trait]
impl ProviderFacade for AnthropicFacade {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn api_format(&self) -> ApiFormat {
        ApiFormat::Anthropic
    }

    fn auth_headers(&self, api_key: &str) -> Vec<(String, String)> {
        vec![
            ("x-api-key".into(), api_key.into()),
            ("anthropic-version".into(), "2023-06-01".into()),
        ]
    }

    fn thinking_facade(&self, model: &ModelInfo) -> Box<dyn ThinkingConfigFacade> {
        if model.reasoning {
            Box::new(ClaudeThinkingFacade)
        } else {
            Box::new(NoOpThinkingFacade)
        }
    }

    fn system_prompt_facade(&self, model: &ModelInfo) -> Box<dyn SystemPromptFacade> {
        // Check if this is OAuth or API key based (from credentials)
        // For now, default to API key style
        Box::new(ClaudeApiKeySystemPromptFacade)
    }

    fn transform_request(&self, model: &ModelInfo, request: GenericRequest) -> ProviderRequest {
        // Transform to Anthropic Messages API format
        ProviderRequest::Anthropic(AnthropicRequest {
            model: model.id.clone(),
            messages: request.messages,
            max_tokens: request.max_tokens.unwrap_or(model.limit.output),
            // ... other fields
        })
    }

    fn model_options(&self, model: &ModelInfo) -> serde_json::Value {
        let mut options = serde_json::json!({});

        // Add beta headers for certain features
        if model.reasoning {
            options["anthropic-beta"] = serde_json::json!("extended-thinking-2025-01-24");
        }

        options
    }
}
```

---

## CLI Integration

### Commands

```bash
# List all providers
codelet models --providers

# List models for a provider
codelet models anthropic
codelet models google

# Search/filter models
codelet models --reasoning           # Only models with reasoning
codelet models --vision              # Only models with vision
codelet models --search "claude"     # Fuzzy search

# Refresh cache
codelet models --refresh

# Show model details
codelet models anthropic/claude-sonnet-4-20250514 --verbose

# Select model for session
codelet --model sonnet "help me code"
codelet --model anthropic/claude-opus-4-5 "complex task"
codelet --model google/gemini-2.5-pro "analyze this"
```

### Configuration

```jsonc
// codelet.json or ~/.codelet/config.json
{
  // Default model (alias or full path)
  "model": "sonnet",

  // Model for lightweight tasks (summarization, etc.)
  "small_model": "haiku",

  // Provider overrides (merged with models.dev)
  "providers": {
    "anthropic": {
      // Add custom models not in models.dev
      "models": {
        "my-fine-tuned-model": {
          "id": "my-fine-tuned-model",
          "name": "My Fine-Tuned Claude",
          "reasoning": false,
          "tool_call": true,
          "limit": { "context": 100000, "output": 4096 }
        }
      }
    }
  },

  // Custom aliases
  "aliases": {
    "fast": "google/gemini-2.0-flash",
    "smart": "anthropic/claude-opus-4-5"
  },

  // Disable specific providers
  "disabled_providers": ["azure", "amazon-bedrock"],

  // Only enable specific providers (whitelist mode)
  "enabled_providers": ["anthropic", "google", "openai"]
}
```

---

## Integration with Existing Facades

### Current State

```
codelet/tools/src/facade/
├── system_prompt.rs      # ClaudeOAuthSystemPromptFacade, GeminiSystemPromptFacade, etc.
├── thinking_config.rs    # ClaudeThinkingFacade, Gemini25ThinkingFacade, etc.
├── web_search.rs         # ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade
├── file_ops.rs           # GeminiReadFileFacade, etc.
└── ...
```

### Integration Points

1. **ThinkingConfigFacade** - Already model-aware, just needs to be selected based on `ModelInfo.reasoning`

2. **SystemPromptFacade** - Selected based on provider + auth method

3. **ToolFacade** - Selected based on provider, capabilities may filter available tools

### Refactoring

```rust
// In agent initialization

async fn create_agent(model_string: &str) -> Result<Agent, Error> {
    let registry = ModelRegistry::global().await;
    let (provider_id, model_id) = registry.parse_model_string(model_string)?;
    let model_info = registry.get_model(&provider_id, &model_id)?;

    let provider_facade = ProviderFacade::for_provider(&provider_id)?;

    let thinking_facade = provider_facade.thinking_facade(&model_info);
    let system_prompt_facade = provider_facade.system_prompt_facade(&model_info);
    let tool_facades = get_tool_facades(&provider_id, &model_info);

    Agent::builder()
        .model(&model_info.id)
        .system_prompt(system_prompt_facade.format_for_api(&preamble))
        .thinking_config(thinking_facade.request_config(ThinkingLevel::Medium))
        .tools(tool_facades)
        .context_limit(model_info.limit.context)
        .build()
}
```

---

## Implementation Phases

### Phase 1: Core Infrastructure
- [ ] Implement `ModelCache` with TTL and refresh
- [ ] Implement `ModelsDevResponse` types with serde
- [ ] Implement `ModelRegistry` with basic lookup
- [ ] Add `--model` flag to CLI
- [ ] Add `codelet models` command

### Phase 2: Provider Facades
- [ ] Define `ProviderFacade` trait
- [ ] Implement `AnthropicFacade`
- [ ] Implement `GoogleFacade`
- [ ] Implement `OpenAIFacade`
- [ ] Implement `OpenAICompatibleFacade` for generic providers

### Phase 3: Integration
- [ ] Connect `ProviderFacade` to existing thinking/system prompt facades
- [ ] Update agent initialization to use `ModelRegistry`
- [ ] Update `ProviderManager` to delegate to new system
- [ ] Wire up tool facade selection based on provider

### Phase 4: Configuration & UX
- [ ] Implement alias system
- [ ] Add `codelet.json` config support
- [ ] Add fuzzy model search
- [ ] Add `--verbose` model details
- [ ] Add capability filtering (`--reasoning`, `--vision`)

### Phase 5: Polish
- [ ] Error messages with suggestions
- [ ] Tab completion for model names
- [ ] Cost estimation display
- [ ] Model comparison view

---

## Benefits of This Approach

1. **Always Current** - New models appear automatically from models.dev
2. **No Maintenance** - No hardcoded lists to update
3. **Unified Source of Truth** - models.dev maintains pricing, limits, capabilities
4. **Extensible** - Easy to add new providers via facade pattern
5. **Offline Capable** - Cache allows offline use with stale data
6. **Type Safe** - Rust types validate models.dev schema
7. **Facade Separation** - Provider behavior separate from model data

---

## Open Questions

1. **Fallback on network failure** - Use embedded snapshot as last resort for first-run?
2. **Model validation** - Validate selected model exists before API call?
3. **Capability requirements** - Should codelet require tool_call=true?
4. **Custom providers** - Support for self-hosted/private models?
