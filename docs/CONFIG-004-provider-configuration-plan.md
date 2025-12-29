# Provider Configuration System - Ultrathink Plan

## Overview

This document outlines the architecture for a comprehensive provider configuration system that:
1. Stores provider settings in `~/.fspec/fspec-config.json`
2. Stores credentials securely in `~/.fspec/credentials/`
3. Reuses existing config functions (DRY/SOLID)
4. Integrates with the model selection screen for API key management
5. Programmatically passes configuration to codelet (Rust) via NAPI/TypeScript

## Current State Analysis

### Existing Infrastructure

**Configuration System (`src/utils/config.ts`)**:
- `getFspecUserDir()` - Returns `~/.fspec` path
- `loadConfig(cwd)` - Loads and merges user + project configs
- `writeConfig(scope, config, cwd)` - Writes to 'user' or 'project' scope
- Config path: `~/.fspec/fspec-config.json`

**Provider Credentials (`codelet/providers/src/credentials.rs`)**:
```rust
pub struct ProviderCredentials {
    pub claude_available: bool,
    pub openai_available: bool,
    pub codex_available: bool,
    pub gemini_available: bool,
}
```
- Detection via environment variables only
- No programmatic configuration support

**Model Selection (`src/tui/components/AgentModal.tsx`)**:
- Uses `modelsListAll()` from codelet-napi
- Filters by `toolCall: true`
- Persists `tui.lastUsedModel`
- No API key configuration UI

**Codelet-NAPI (`codelet/napi/`)**:
- `CodeletSession` class
- No credential passing mechanism
- Provider selection via environment only

## Proposed Architecture

### 1. Configuration Schema Extension

**`~/.fspec/fspec-config.json`** additions:

```json
{
  "providers": {
    "anthropic": {
      "enabled": true,
      "defaultModel": "claude-sonnet-4",
      "baseUrl": null
    },
    "openai": {
      "enabled": true,
      "defaultModel": "gpt-4-turbo",
      "baseUrl": "https://api.openai.com/v1"
    },
    "google": {
      "enabled": true,
      "defaultModel": "gemini-2.0-flash",
      "baseUrl": null
    },
    "openrouter": {
      "enabled": true,
      "defaultModel": "anthropic/claude-3.5-sonnet",
      "baseUrl": "https://openrouter.ai/api/v1"
    }
  },
  "tui": {
    "lastUsedModel": "anthropic/claude-sonnet-4"
  }
}
```

### 2. Credentials Store

**Directory**: `~/.fspec/credentials/`

**File**: `credentials.json` (with 600 permissions)

```json
{
  "version": 1,
  "providers": {
    "anthropic": {
      "apiKey": "sk-ant-...",
      "lastUpdated": "2025-01-15T10:30:00Z"
    },
    "openai": {
      "apiKey": "sk-...",
      "lastUpdated": "2025-01-15T10:30:00Z"
    },
    "google": {
      "apiKey": "AIza...",
      "lastUpdated": "2025-01-15T10:30:00Z"
    },
    "openrouter": {
      "apiKey": "sk-or-...",
      "lastUpdated": "2025-01-15T10:30:00Z"
    }
  }
}
```

**Security considerations**:
- File permissions: 600 (owner read/write only)
- Directory permissions: 700 (owner rwx only)
- Warning on insecure permissions
- No encryption initially (OS keychain integration as future enhancement)

### 3. TypeScript Config Extensions

**File**: `src/utils/credentials.ts` (new)

```typescript
import { getFspecUserDir, loadConfig, writeConfig } from './config.js';
import fs from 'fs';
import path from 'path';

interface ProviderCredential {
  apiKey: string;
  lastUpdated: string;
}

interface CredentialsStore {
  version: number;
  providers: Record<string, ProviderCredential>;
}

interface ProviderConfig {
  enabled: boolean;
  defaultModel?: string;
  baseUrl?: string;
}

// Reuses getFspecUserDir from config.ts (DRY)
export function getCredentialsPath(): string {
  return path.join(getFspecUserDir(), 'credentials', 'credentials.json');
}

export async function loadCredentials(): Promise<CredentialsStore> {
  const credPath = getCredentialsPath();
  if (!fs.existsSync(credPath)) {
    return { version: 1, providers: {} };
  }
  const content = await fs.promises.readFile(credPath, 'utf-8');
  return JSON.parse(content);
}

export async function saveCredential(
  provider: string,
  apiKey: string
): Promise<void> {
  const store = await loadCredentials();
  store.providers[provider] = {
    apiKey,
    lastUpdated: new Date().toISOString()
  };

  const credDir = path.dirname(getCredentialsPath());
  await fs.promises.mkdir(credDir, { recursive: true, mode: 0o700 });
  await fs.promises.writeFile(
    getCredentialsPath(),
    JSON.stringify(store, null, 2),
    { mode: 0o600 }
  );
}

export async function deleteCredential(provider: string): Promise<void> {
  const store = await loadCredentials();
  delete store.providers[provider];
  await fs.promises.writeFile(
    getCredentialsPath(),
    JSON.stringify(store, null, 2),
    { mode: 0o600 }
  );
}

// Get merged provider config (credentials + settings)
export async function getProviderConfig(provider: string): Promise<{
  apiKey?: string;
  enabled: boolean;
  defaultModel?: string;
  baseUrl?: string;
}> {
  const config = loadConfig(process.cwd());
  const credentials = await loadCredentials();

  const providerSettings = config.providers?.[provider] ?? { enabled: true };
  const providerCreds = credentials.providers[provider];

  return {
    ...providerSettings,
    apiKey: providerCreds?.apiKey
  };
}
```

### 4. Codelet-NAPI Extensions

#### 4.1 Rust Types (`codelet/napi/src/types.rs`)

```rust
#[napi(object)]
pub struct NapiProviderConfig {
    pub provider_id: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub enabled: bool,
    pub default_model: Option<String>,
}

#[napi(object)]
pub struct NapiCredentialsConfig {
    pub providers: Vec<NapiProviderConfig>,
}
```

#### 4.2 Session Creation with Config (`codelet/napi/src/session.rs`)

```rust
#[napi]
impl CodeletSession {
    // Existing: auto-detect from environment
    #[napi(constructor)]
    pub fn new(provider_name: Option<String>) -> Result<Self>;

    // NEW: Create with explicit config (programmatic credentials)
    #[napi(factory)]
    pub async fn new_with_credentials(
        model_string: String,
        credentials: NapiProviderConfig,
    ) -> Result<Self>;

    // NEW: Set credentials at runtime
    #[napi]
    pub fn set_provider_credential(
        &mut self,
        provider_id: String,
        api_key: String,
    ) -> Result<()>;
}
```

#### 4.3 Credential Source Priority

When creating a session, credentials are resolved in this order:
1. **Explicit config** passed to `new_with_credentials()` or `set_provider_credential()`
2. **Config file** from `~/.fspec/credentials/credentials.json`
3. **Environment variables** (`ANTHROPIC_API_KEY`, etc.)
4. **.env file** via dotenvy

#### 4.4 Provider Manager Updates (`codelet/providers/src/manager.rs`)

```rust
impl ProviderManager {
    // Existing: detect from environment
    pub fn new() -> Result<Self, ProviderError>;

    // NEW: Create with explicit credentials
    pub fn with_credentials(config: ProviderCredentialsConfig) -> Result<Self, ProviderError>;
}

// NEW: Configuration structure for programmatic use
pub struct ProviderCredentialsConfig {
    pub anthropic: Option<ProviderCredential>,
    pub openai: Option<ProviderCredential>,
    pub google: Option<ProviderCredential>,
    pub openrouter: Option<ProviderCredential>,
}

pub struct ProviderCredential {
    pub api_key: String,
    pub base_url: Option<String>,
}
```

### 5. Model Selection UI Enhancement

**File**: `src/tui/components/AgentModal.tsx`

New features:
1. **Provider Settings Section** (Tab key to access)
2. **API Key Input** with masked display
3. **Enable/Disable Toggle** per provider
4. **Base URL Override** for OpenAI-compatible endpoints
5. **Connection Test** button per provider

```
┌─────────────────────────────────────────────────────────┐
│ Model Selection                         [Tab: Settings] │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ ▼ Anthropic (configured)                               │
│   • Claude Sonnet 4 (200k context)                     │
│   • Claude Opus 4.5 (200k context)                     │
│                                                         │
│ ▼ OpenAI (not configured)                              │
│   ⚠ API key required                                   │
│                                                         │
│ ▼ Google (configured)                                  │
│   • Gemini 2.0 Flash (1M context)                      │
│                                                         │
│ ▼ OpenRouter (configured)                              │
│   • (uses OpenRouter model catalog)                    │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ [Enter] Select  [Tab] Settings  [Esc] Cancel           │
└─────────────────────────────────────────────────────────┘
```

**Settings View**:

```
┌─────────────────────────────────────────────────────────┐
│ Provider Settings                      [Tab: Models]    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Anthropic                                              │
│ ├ API Key: sk-ant-••••••••••••••vX8D [Edit]            │
│ ├ Status: ✓ Connected                                  │
│ └ Default Model: claude-sonnet-4                       │
│                                                         │
│ OpenAI                                                 │
│ ├ API Key: [Enter API key...]                          │
│ ├ Status: ⚠ Not configured                             │
│ ├ Base URL: https://api.openai.com/v1                  │
│ └ Default Model: gpt-4-turbo                           │
│                                                         │
│ Google                                                 │
│ ├ API Key: AIza••••••••••••••••y9Z [Edit]              │
│ ├ Status: ✓ Connected                                  │
│ └ Default Model: gemini-2.0-flash                      │
│                                                         │
│ OpenRouter                                             │
│ ├ API Key: [Enter API key...]                          │
│ ├ Status: ⚠ Not configured                             │
│ ├ Base URL: https://openrouter.ai/api/v1               │
│ └ Default Model: anthropic/claude-3.5-sonnet           │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ [Enter] Edit  [t] Test  [d] Delete  [Tab] Models       │
└─────────────────────────────────────────────────────────┘
```

### 6. OpenRouter Integration

OpenRouter provides access to many models via a unified API. This is important for:
- Access to models not directly available (Llama, Mistral, etc.)
- Fallback when primary providers are unavailable
- Cost optimization (choose cheaper models for simple tasks)

**Configuration**:
```json
{
  "providers": {
    "openrouter": {
      "enabled": true,
      "baseUrl": "https://openrouter.ai/api/v1",
      "defaultModel": "anthropic/claude-3.5-sonnet"
    }
  }
}
```

**Implementation notes**:
- Uses OpenAI-compatible API
- Requires special headers: `HTTP-Referer`, `X-Title`
- Model IDs use `provider/model` format

### 7. Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              User Action                                 │
│                    (Configure API key in Settings)                       │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                     TypeScript: credentials.ts                           │
│                                                                          │
│   saveCredential('anthropic', 'sk-ant-...')                             │
│   → Write to ~/.fspec/credentials/credentials.json                       │
│   → Set file permissions 600                                             │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    TypeScript: AgentModal.tsx                            │
│                                                                          │
│   1. Load credentials from credentials.ts                                │
│   2. Load provider config from config.ts                                 │
│   3. Merge: { apiKey, enabled, defaultModel, baseUrl }                   │
│   4. Create session: CodeletSession.new_with_credentials(...)            │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        NAPI Layer (Rust)                                 │
│                                                                          │
│   NapiProviderConfig {                                                   │
│     provider_id: "anthropic",                                            │
│     api_key: Some("sk-ant-..."),                                         │
│     base_url: None,                                                      │
│     enabled: true,                                                       │
│     default_model: Some("claude-sonnet-4")                               │
│   }                                                                      │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Rust: ProviderManager                                 │
│                                                                          │
│   ProviderManager::with_credentials(config)                              │
│   → Creates provider with explicit API key                               │
│   → Bypasses environment variable detection                              │
│   → Falls back to env vars if config is None                             │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Provider Implementation                           │
│                                                                          │
│   ClaudeProvider::new_with_config(api_key, base_url, model)             │
│   OpenAIProvider::new_with_config(api_key, base_url, model)             │
│   GeminiProvider::new_with_config(api_key, base_url, model)             │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8. Credential Source Priority Chain

```
┌───────────────────────────────────────────────────────────────────────┐
│                     Credential Resolution Order                        │
├───────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   1. EXPLICIT CONFIG (passed via NAPI)                                │
│      ↓ if not provided                                                 │
│   2. CREDENTIALS FILE (~/.fspec/credentials/credentials.json)          │
│      ↓ if not found                                                    │
│   3. ENVIRONMENT VARIABLE (ANTHROPIC_API_KEY, etc.)                    │
│      ↓ if not set                                                      │
│   4. .ENV FILE (via dotenvy)                                           │
│      ↓ if not found                                                    │
│   5. OAUTH/KEYCHAIN (Codex only - ~/.codex/auth.json)                  │
│      ↓ if not available                                                │
│   6. ERROR: Provider not available                                     │
│                                                                        │
└───────────────────────────────────────────────────────────────────────┘
```

### 9. Implementation Phases

#### Phase 1: Core Infrastructure
1. Create `src/utils/credentials.ts` with CRUD operations
2. Add TypeScript types for provider configuration
3. Update `src/utils/config.ts` with `providers` schema
4. Add file permission handling for credentials

#### Phase 2: Codelet-NAPI Integration
1. Add `NapiProviderConfig` type to Rust NAPI bindings
2. Implement `CodeletSession::new_with_credentials()`
3. Update `ProviderManager` to accept explicit credentials
4. Add `set_provider_credential()` for runtime updates

#### Phase 3: Provider Manager Updates
1. Add `with_credentials()` factory to `ProviderManager`
2. Update each provider constructor to accept explicit config
3. Implement credential source priority chain
4. Add validation for API keys (format, basic connectivity test)

#### Phase 4: UI Integration
1. Add Settings tab to `AgentModal.tsx`
2. Implement API key input with masking
3. Add connection test functionality
4. Implement enable/disable toggles
5. Add base URL configuration for OpenAI-compatible providers

#### Phase 5: OpenRouter Support
1. Add OpenRouter provider to provider types
2. Implement OpenAI-compatible adapter with OpenRouter headers
3. Add model discovery from OpenRouter API
4. Integrate with model selection UI

### 10. Testing Strategy

**Unit Tests**:
- `credentials.test.ts`: CRUD operations, file permissions
- `config.test.ts`: Provider schema validation, merge logic
- `credentials.rs`: Credential resolution chain

**Integration Tests**:
- NAPI credential passing end-to-end
- Session creation with explicit credentials
- Provider fallback chain verification

**Manual Testing**:
- Settings UI flow
- API key entry and persistence
- Connection testing per provider

### 11. Security Considerations

1. **File Permissions**: Enforce 600 on credentials file, 700 on directory
2. **No Logging**: Never log API keys, even partially
3. **Memory**: Clear credentials from memory when session ends
4. **Validation**: Warn on insecure file permissions
5. **Future**: OS keychain integration (macOS Keychain, Windows Credential Manager, Linux Secret Service)

### 12. Migration Path

For existing users with environment variables:
1. Environment variables continue to work (backward compatible)
2. UI shows "Configured via environment" for env-based credentials
3. Optional migration: "Save to config" button copies env to credentials file
4. Documentation on transitioning from `.env` to config-based approach

### 13. Related Files

**New Files**:
- `src/utils/credentials.ts` - Credential management
- `codelet/napi/src/provider_config.rs` - NAPI provider config types

**Modified Files**:
- `src/utils/config.ts` - Add provider config types
- `src/tui/components/AgentModal.tsx` - Settings UI
- `codelet/napi/src/session.rs` - new_with_credentials()
- `codelet/napi/src/types.rs` - NapiProviderConfig
- `codelet/napi/index.d.ts` - TypeScript declarations
- `codelet/providers/src/manager.rs` - with_credentials()
- `codelet/providers/src/credentials.rs` - ProviderCredentialsConfig
- `codelet/providers/src/claude.rs` - new_with_config()
- `codelet/providers/src/openai.rs` - new_with_config()
- `codelet/providers/src/gemini.rs` - new_with_config()

### 14. Open Questions

1. Should we support project-level credentials (spec/credentials/) for team sharing?
2. Should credentials be encrypted at rest? (adds complexity)
3. Should we validate API keys on save (requires network)?
4. Should OpenRouter be a first-class provider or OpenAI-compatible extension?
5. How to handle credential rotation notifications?

### 15. Success Criteria

- [ ] API keys can be configured via Settings UI
- [ ] Credentials persist in `~/.fspec/credentials/credentials.json`
- [ ] File permissions are enforced (600/700)
- [ ] Credentials can be passed programmatically to codelet via NAPI
- [ ] Environment variables continue to work (backward compatible)
- [ ] Connection test validates credentials before saving
- [ ] Model selection shows configured/not configured status per provider
