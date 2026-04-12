# AST Research: Context Window and Model Selection Flow

**Date:** 2026-04-11
**Work Unit:** MODEL-005
**Method:** AstGrep structural search across Rust + TypeScript

---

## 1. ProviderManager Key Methods (codelet/providers/src/manager.rs)

### context_window() — Line 622
```
pub fn context_window(&self) -> usize — returns compile-time constant per provider type
```
No model-specific resolution. Returns hardcoded values.

### max_output_tokens() — Line 638
```
pub fn max_output_tokens(&self) -> usize — reads OPENAI_MAX_OUTPUT_TOKENS env var for OpenAI, constants for others
```
Has env var override only for OpenAI provider (PROV-039).

### select_model() — Line 223
```
pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError>
```
Validates model via registry, sets `current_provider` and `selected_model`. Does NOT extract or store LimitInfo from ModelInfo.

### set_model_direct() — Line 281
```
pub fn set_model_direct(&mut self, provider_id: &str, model_id: &str) -> Result<(), ProviderError>
```
Bypasses registry for profile models. Only sets `current_provider` and `selected_model`. No context_window params.

### for_testing() — Line 659
```
pub fn for_testing(provider: ProviderType) -> Self
```
Creates test manager with no credentials. No context_window params.

## 2. ProviderManager Struct (Line 81-88)

```rust
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    model_registry: Option<ModelRegistry>,
    selected_model: Option<String>,
}
```

Missing fields: `model_context_window: Option<usize>`, `model_max_output_tokens: Option<usize>`.

## 3. NAPI Layer (codelet/napi/src/session_manager.rs)

### session_set_model — Line 6424
```rust
pub async fn session_set_model(session_id: String, provider_id: String, model_id: String) -> Result<()>
```
No context_window or max_output_tokens params.

### session_set_model_profile — Line 6462
```rust
pub async fn session_set_model_profile(session_id: String, provider_id: String, model_id: String) -> Result<()>
```
No context_window or max_output_tokens params.

## 4. TypeScript Layer (src/tui/services/modelSelectionService.ts)

### sessionSetModel calls — Line 110
```typescript
await sessionSetModel(sessionId, selection.providerId, selection.modelId)
```
Does NOT pass `selection.contextWindow` or `selection.maxOutput`.

### sessionSetModelProfile calls — Lines 96, 103
```typescript
await sessionSetModelProfile(sessionId, selection.providerId, selection.modelId)
```
Does NOT pass context params either.

## 5. NAPI Type Declarations (codelet/napi/index.d.ts)

```typescript
export declare function sessionSetModel(sessionId: string, providerId: string, modelId: string): Promise<void>;
export declare function sessionSetModelProfile(sessionId: string, providerId: string, modelId: string): Promise<void>;
```
No optional context_window/max_output_tokens params.

## 6. Provider Constants

| Provider | CONTEXT_WINDOW | MAX_OUTPUT_TOKENS | File |
|----------|---------------|-------------------|------|
| Claude | 200,000 | 8,192 | claude.rs:42-45 |
| OpenAI | 128,000 | 4,096 | openai.rs:31,34 |
| Gemini | 1,000,000 | 8,192 | gemini.rs:20,23 |
| Codex | 272,000 | 4,096 | codex/mod.rs:42,45 |
| Z.AI | 128,000 | 8,192 | zai.rs:30,33 |
| Copilot | 200,000 | 4,096 | copilot/mod.rs:63,71 |

## 7. LimitInfo Struct (codelet/providers/src/models/types.rs:161)

```rust
pub struct LimitInfo {
    pub context: u32,
    pub output: u32,
}
```

This is what ModelInfo.limit contains — the per-model data from models.dev that needs to be propagated.

## Key Gap

ModelInfo.limit (LimitInfo) is available after select_model() validates the model, but its context/output values are never stored on ProviderManager. The data exists but isn't wired through.
