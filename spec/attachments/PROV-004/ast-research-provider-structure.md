# AST Research: Provider Structure Analysis

## Research Query
Analyzed the provider structure in `codelet/providers/src/` to understand the pattern for adding Z.AI provider.

## Provider Structs Found

```
codelet/providers/src/gemini.rs:30:1:pub struct GeminiProvider
codelet/providers/src/claude.rs:83:1:pub struct ClaudeProvider
codelet/providers/src/openai.rs:29:1:pub struct OpenAIProvider
codelet/providers/src/codex/mod.rs:31:1:pub struct CodexProvider
codelet/providers/src/manager.rs:59:1:pub struct ProviderManager
codelet/providers/src/models/registry.rs:16:1:pub struct ModelRegistry
```

## ProviderType Enum (manager.rs)

```rust
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
}
```

### FromStr Implementation
```rust
"claude" => Ok(ProviderType::Claude),
"openai" => Ok(ProviderType::OpenAI),
"codex" => Ok(ProviderType::Codex),
"gemini" => Ok(ProviderType::Gemini),
```

## Files to Modify for Z.AI

1. **codelet/providers/src/lib.rs** - Add `pub mod zai;` and re-export `ZAIProvider`
2. **codelet/providers/src/manager.rs** - Add `ProviderType::ZAI` variant
3. **codelet/providers/src/credentials.rs** - Add `has_zai()` method
4. **New file: codelet/providers/src/zai.rs** - Create ZAIProvider struct

## Implementation Pattern (from GeminiProvider)

```rust
pub struct GeminiProvider {
    completion_model: gemini::completion::CompletionModel,
    rig_client: gemini::Client,
    model_name: String,
}

impl GeminiProvider {
    pub fn new() -> Result<Self, ProviderError> {
        let api_key = detect_credential_from_env("gemini", &["GOOGLE_GENERATIVE_AI_API_KEY"])?;
        // ...
    }
}
```

## Z.AI Equivalent

```rust
pub struct ZAIProvider {
    // Use OpenAI client with custom base_url
    rig_client: openai::Client,
    model_name: String,
}

impl ZAIProvider {
    pub fn new() -> Result<Self, ProviderError> {
        let api_key = detect_credential_from_env("zai", &["ZAI_API_KEY"])?;
        // Configure with https://api.z.ai/api/paas/v4
    }
}
```

## TypeScript Files to Update

From `src/utils/provider-config.ts`:
- Add 'zai' to `SUPPORTED_PROVIDERS` array
- Add registry entry with `envVar: 'ZAI_API_KEY'`

From `src/tui/components/AgentView.tsx`:
- Add mapping for 'zai' provider ID
