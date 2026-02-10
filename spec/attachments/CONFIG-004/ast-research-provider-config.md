# AST Research: Provider Configuration System

## Summary

Analysis of existing infrastructure for CONFIG-004 implementation.

## 1. Config System (`src/utils/config.ts`)

### Key Functions

| Function | Line | Purpose |
|----------|------|---------|
| `getFspecUserDir()` | 13 | Returns `~/.fspec` path |
| `deepMerge()` | 21 | Deep merge config objects |
| `loadConfigFile()` | 44 | Load JSON config from path |
| `loadConfig()` | 79 | Merge user + project configs |
| `writeConfig()` | 99 | Write config to user/project scope |

### Integration Points

- `getFspecUserDir()` - **REUSE** for credentials path
- `loadConfig()` - **EXTEND** to include providers schema
- `writeConfig()` - **REUSE** for provider settings

## 2. Credentials System (`codelet/providers/src/credentials.rs`)

### Current Implementation

| Function | Line | Purpose |
|----------|------|---------|
| `detect()` | 18 | Check env vars for credentials |
| `has_any()` | 29 | Check if any provider available |
| `has_claude()` | 37 | Check ANTHROPIC_API_KEY |
| `has_openai()` | 42 | Check OPENAI_API_KEY |
| `has_codex()` | 47 | Check ~/.codex/auth.json |
| `has_gemini()` | 52 | Check GOOGLE_GENERATIVE_AI_API_KEY |
| `available_providers()` | 57 | List available providers |
| `has_codex_auth()` | 77 | Check Codex OAuth tokens |

### Required Changes

- **ADD**: `detect_from_config()` - Load from config file
- **ADD**: `with_explicit_credentials()` - Accept programmatic credentials
- **MODIFY**: `detect()` - Add config file to resolution chain

## 3. Session System (`codelet/napi/src/session.rs`)

### Current Implementation

| Function | Line | Purpose |
|----------|------|---------|
| `new()` | 50 | Create session with env detection |
| `new_with_model()` | 82 | Create session with specific model |
| `switch_provider()` | 380 | Switch to different provider |
| `select_model()` | 408 | Select model within provider |

### Required Changes

- **ADD**: `new_with_credentials()` - Factory accepting explicit credentials
- **ADD**: `set_provider_credential()` - Runtime credential update
- **MODIFY**: `new_with_model()` - Accept optional credentials config

## 4. Provider Support Matrix

Based on rig analysis, need to support 19 providers:

| Provider | Auth Method | Special Config |
|----------|-------------|----------------|
| OpenAI | Bearer | baseUrl |
| Anthropic | x-api-key | version headers |
| Cohere | Bearer | - |
| Gemini | Query param | - |
| Mistral | Bearer | - |
| xAI | Bearer | - |
| Together | Bearer | - |
| HuggingFace | Bearer | - |
| OpenRouter | Bearer | HTTP-Referer |
| Groq | Bearer | - |
| Ollama | None | baseUrl (local) |
| DeepSeek | Bearer | - |
| Perplexity | Bearer | - |
| Moonshot | Bearer | - |
| Hyperbolic | Bearer | - |
| Mira | Bearer | - |
| Galadriel | Bearer | fine-tune key |
| Azure OpenAI | api-key header | endpoint, apiVersion |
| Voyage AI | Bearer | embeddings only |

## 5. Files to Modify

### TypeScript
- `src/utils/config.ts` - Add providers schema
- `src/utils/credentials.ts` - NEW: Credential CRUD
- `src/tui/components/AgentModal.tsx` - Settings UI

### Rust (codelet-napi)
- `codelet/napi/src/session.rs` - new_with_credentials()
- `codelet/napi/src/types.rs` - NapiProviderConfig
- `codelet/napi/index.d.ts` - TypeScript declarations

### Rust (codelet-providers)
- `codelet/providers/src/credentials.rs` - Config file support
- `codelet/providers/src/manager.rs` - with_credentials()
- Individual provider files - new_with_config()

## 6. Test Coverage Required

### Unit Tests
- `credentials.test.ts` - CRUD, permissions
- `config.test.ts` - Provider schema

### Integration Tests
- NAPI credential passing
- Priority chain verification
- Provider fallback

## 7. Thinking Configuration Integration

The thinking/reasoning config system uses different provider identifiers than the provider registry.

### Current State

| File | Purpose |
|------|---------|
| `codelet/napi/src/thinking_config.rs` | NAPI bindings for thinking config |
| `codelet/tools/src/facade/thinking_config.rs` | Rust facades for provider-specific thinking |
| `src/utils/thinkingLevel.ts` | TypeScript thinking level detection |
| `src/tui/components/ThinkingLevelDialog.tsx` | UI for `/thinking` command |

### Provider Mapping Issue

**Problem**: `getThinkingConfig(provider, level)` expects provider strings like:
- `"claude"`, `"claude-3"`, `"claude-sonnet"` → `ClaudeThinkingFacade`
- `"gemini-3"`, `"gemini-3-flash"` → `Gemini3ThinkingFacade`
- `"gemini-2.5"`, `"gemini-2.5-pro"` → `Gemini25ThinkingFacade`

**But**: `AgentView.tsx` passes `currentProvider` which is the provider ID like `"anthropic"`, `"google"`.

### Required Changes

| File | Change |
|------|--------|
| `src/utils/provider-config.ts` | Add `thinkingConfigProvider` field to registry |
| `src/utils/provider-config.ts` | Add `getThinkingConfigProvider(providerId, modelId)` helper |
| `src/tui/components/AgentView.tsx` | Map provider to thinking config provider before calling `getThinkingConfig()` |

### Provider to Thinking Config Mapping

| Provider ID | Model Pattern | Thinking Config Provider |
|-------------|--------------|-------------------------|
| `anthropic` | any | `claude` |
| `google` | `gemini-3-*` | `gemini-3` |
| `google` | `gemini-2.5-*`, `gemini-2.0-*` | `gemini-2.5` |
| All others | any | `null` (not supported) |

## 8. Implementation Order

1. TypeScript credentials module
2. Rust NAPI types
3. Provider manager with_credentials()
4. Session new_with_credentials()
5. Provider registry with thinking config mapping
6. UI Settings tab
