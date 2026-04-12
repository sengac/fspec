# Model System Architecture Analysis

**Date:** 2026-04-11
**Purpose:** Architecture investigation for MODEL-004 (Facade Dispatch Chain) and MODEL-005 (Context Window Flow)
**Method:** GraphSearch AST analysis + DeepSearch + targeted file reads

---

## Table of Contents

1. [Context Window Flow (MODEL-005)](#1-context-window-flow-model-005)
2. [Facade Dispatch Chain (MODEL-004)](#2-facade-dispatch-chain-model-004)
3. [Model Initialization Pipeline](#3-model-initialization-pipeline)
4. [Compaction Threshold Deep Dive](#4-compaction-threshold-deep-dive)
5. [NAPI Model Switch Call Chain](#5-napi-model-switch-call-chain)
6. [Key Findings & Gaps](#6-key-findings--gaps)

---

## 1. Context Window Flow (MODEL-005)

### 1.1 Per-Provider Context Window Constants

Each provider defines compile-time constants:

| Provider | `CONTEXT_WINDOW` | `MAX_OUTPUT_TOKENS` | Source File |
|----------|------------------|---------------------|-------------|
| Claude | 200,000 | 8,192 | `codelet/providers/src/claude.rs:42-45` |
| OpenAI | 128,000 (default) | 4,096 (default) | `codelet/providers/src/openai.rs:24-34` |
| Gemini | 1,000,000 | 8,192 | `codelet/providers/src/gemini.rs:20-23` |
| Codex | 272,000 | 4,096 | `codelet/providers/src/codex/mod.rs:42-45` |
| Z.AI | 128,000 | 8,192 | `codelet/providers/src/zai.rs:30-33` |
| GitHub Copilot | 200,000 | 4,096 | `codelet/providers/src/copilot/mod.rs:63-71` |

### 1.2 ProviderManager::context_window() — The Central Dispatcher

**File:** `codelet/providers/src/manager.rs:622-631`

```rust
pub fn context_window(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::CONTEXT_WINDOW,
        ProviderType::OpenAI => openai::CONTEXT_WINDOW,
        ProviderType::Gemini => gemini::CONTEXT_WINDOW,
        ProviderType::Codex => codex::CONTEXT_WINDOW,
        ProviderType::ZAI => zai::CONTEXT_WINDOW,
        ProviderType::GitHubCopilot => copilot::CONTEXT_WINDOW,
    }
}
```

**CRITICAL GAP:** This function returns **compile-time constants only**. It does NOT read `OPENAI_CONTEXT_WINDOW` env var at runtime. Compare with `max_output_tokens()` which DOES read `OPENAI_MAX_OUTPUT_TOKENS` at runtime for OpenAI.

### 1.3 ProviderManager::max_output_tokens() — Has Runtime Override

**File:** `codelet/providers/src/manager.rs:638-653`

```rust
pub fn max_output_tokens(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::MAX_OUTPUT_TOKENS,
        ProviderType::OpenAI => {
            // PROV-039: Read OPENAI_MAX_OUTPUT_TOKENS env var at runtime
            std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(openai::MAX_OUTPUT_TOKENS)
        }
        ProviderType::Gemini => gemini::MAX_OUTPUT_TOKENS,
        ProviderType::Codex => codex::MAX_OUTPUT_TOKENS,
        ProviderType::ZAI => zai::MAX_OUTPUT_TOKENS,
        ProviderType::GitHubCopilot => copilot::MAX_OUTPUT_TOKENS,
    }
}
```

**KEY ASYMMETRY:** `max_output_tokens()` reads `OPENAI_MAX_OUTPUT_TOKENS` at runtime (PROV-039), but `context_window()` does NOT read `OPENAI_CONTEXT_WINDOW`. The OpenAI provider instance (`OpenAIProvider`) DOES read `OPENAI_CONTEXT_WINDOW` during construction (line 167-170 of openai.rs), but the ProviderManager dispatch ignores the instance value.

### 1.4 OpenAI Provider Instance — Reads Env Vars at Construction

**File:** `codelet/providers/src/openai.rs:167-175`

```rust
let context_window = std::env::var("OPENAI_CONTEXT_WINDOW")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(DEFAULT_CONTEXT_WINDOW);

let max_output_tokens = std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);
```

The `OpenAIProvider` struct stores `context_window` and `max_output_tokens` as instance fields. Its `LlmProvider` trait impl returns `self.context_window`. However, **ProviderManager::context_window() ignores this** and returns the compile-time constant.

### 1.5 Callers of context_window()

GraphSearch `ast_callers` returned 0 results (cross-crate calls not tracked). Grep-based analysis found:

| Call Site | File | Line | Purpose |
|-----------|------|------|---------|
| stream_loop threshold calc | `cli/src/interactive/stream_loop.rs` | 276 | `context_window as u64` for compaction threshold |
| inject_summary handler | `napi/src/session_manager.rs` | 4917 | Passes to inject_summary handler |
| debug metadata | `napi/src/session_manager.rs` | 7071, 7109 | SessionMetadata for debug capture |
| NAPI model info | `napi/src/session_manager.rs` | 5658 | Reports to TypeScript as `context_window: f64` |

### 1.6 Callers of max_output_tokens()

| Call Site | File | Line | Purpose |
|-----------|------|------|---------|
| stream_loop threshold calc | `cli/src/interactive/stream_loop.rs` | 277 | `max_output_tokens as u64` for usable context |

---

## 2. Facade Dispatch Chain (MODEL-004)

### 2.1 ProviderType Enum

**File:** `codelet/providers/src/manager.rs:21-29`

```rust
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    GitHubCopilot,
}
```

### 2.2 ProviderManager Struct

**File:** `codelet/providers/src/manager.rs:81-88`

```rust
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    model_registry: Option<ModelRegistry>,
    selected_model: Option<String>,
}
```

### 2.3 set_model_direct() — Bypass Registry

**File:** `codelet/providers/src/manager.rs:281-299`

Used for: profile-based models (vLLM, Ollama) and Codex models.

```rust
pub fn set_model_direct(&mut self, provider_id: &str, model_id: &str) -> Result<(), ProviderError> {
    let provider_type = Self::map_provider_id_to_type(provider_id)?;
    // NOTE: Intentionally skips credentials validation.
    self.current_provider = provider_type;
    self.selected_model = Some(model_id.to_string());
    Ok(())
}
```

**Callees:** `map_provider_id_to_type()` only.

### 2.4 select_model() — Registry-Validated

**File:** `codelet/providers/src/manager.rs:223-267`

Used for: cloud providers (anthropic, openai, google).

```rust
pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError> {
    self.credentials = ProviderCredentials::detect();  // PROV-057: stale-cache fix
    let registry = self.model_registry.as_ref()...;
    let (provider_id, model_id) = registry.parse_model_string(model_string)?;
    let provider_type = Self::map_provider_id_to_type(&provider_id)?;
    // Validate credentials
    if !provider_type.has_credentials(&self.credentials) { ... }
    // Validate model exists and has tool_call capability
    let model_info = registry.validate_model_for_use(&provider_id, &model_id)?;
    self.current_provider = provider_type;
    self.selected_model = Some(model_string.to_string());
    Ok(model_info)
}
```

### 2.5 map_provider_id_to_type() — ID to Enum Mapping

**File:** `codelet/providers/src/manager.rs:350-365`

```
"anthropic" → Claude
"openai"    → OpenAI
"google"    → Gemini
"zai"|"z-ai" → ZAI
"codex"     → Codex
"github-copilot"|"copilot" → GitHubCopilot
```

### 2.6 create_rig_agent() — Per-Provider Implementations

Each provider has its own `create_rig_agent()` method with provider-specific tool facades:

| Provider | Tool Strategy | Max Tokens Source |
|----------|--------------|-------------------|
| **Claude** | Native tools (Read, Write, Edit, Bash, Grep, Glob, Ls) + ClaudeWebSearchFacade | `MAX_OUTPUT_TOKENS` constant (8192) |
| **OpenAI** | Native tools (same as Claude) + openai_fspec_tool | `self.max_output_tokens` instance field |
| **Gemini** | Gemini facades (read_file, write_file, replace, run_shell_command, etc.) | `MAX_OUTPUT_TOKENS` constant (8192) |
| **Codex** | Codex facades (shell_command, read_file, apply_patch, shell, exec_command) | Does NOT set .max_tokens() (Codex API rejects it) |
| **Z.AI** | Z.AI facades (read_file, write_file, edit_file, run_command, etc.) | `MAX_OUTPUT_TOKENS` constant (8192) |
| **Copilot** | Native tools (same as Claude/OpenAI) | `copilot::MAX_OUTPUT_TOKENS` constant (4096) |

**IMPORTANT:** Only OpenAI's `create_rig_agent()` uses the instance-level `self.max_output_tokens` (which reads from env var). All others use compile-time constants.

### 2.7 run_with_provider! Macro — NAPI Agent Loop Dispatch

**File:** `codelet/napi/src/session_manager.rs:4197-4262`

The macro:
1. Calls `provider_manager_mut().$getter()` to get the provider instance
2. Gathers MCP tool wrappers
3. Reads session role for preamble
4. Calls `provider.create_rig_agent(session.id, preamble, thinking_config)`
5. Adds MCP tools post-build
6. Wraps in `RigAgent::with_default_depth()`
7. Calls `run_agent_stream_with_images()`

Dispatch arms (line 5166+):
```
"claude"              → run_with_provider!(..., get_claude, ...)
"openai"              → custom (get_openai needs session_id)
"gemini"              → run_with_provider!(..., get_gemini, ...)
"zai"                 → run_with_provider!(..., get_zai, ...)
"codex"               → run_with_provider!(..., get_codex, ...)
"github-copilot"|"copilot" → run_with_provider!(..., get_github_copilot, ...)
```

### 2.8 configureProfileEnvironment — TypeScript → Env Vars

**File:** `src/tui/services/profileEnvironmentService.ts:28-38`

```typescript
function configureProfileEnvironment(config: ProfileConfig): void {
  process.env.OPENAI_BASE_URL = config.baseUrl;
  process.env.OPENAI_API_KEY = config.apiKey;
  if (config.contextWindow) {
    process.env.OPENAI_CONTEXT_WINDOW = String(config.contextWindow);
  }
  if (config.maxOutputTokens) {
    process.env.OPENAI_MAX_OUTPUT_TOKENS = String(config.maxOutputTokens);
  }
}
```

This is called BEFORE the NAPI call so Rust sees the env vars.

---

## 3. Model Initialization Pipeline

### 3.1 initializeModels() — TypeScript Entry Point

**File:** `src/tui/services/modelInitializationService.ts:386-503`

Flow:
1. `loadCloudModels()` → fetches from models.dev cache
2. `buildCloudSections(cloudModels)` → filters by credentials, extracts Codex section
3. `loadProfileSections()` → iterates OpenAI profiles, calls `modelsListLocalOpenai()`
4. Combines: profiles first, then cloud
5. Restores persisted model or selects default
6. Updates Zustand store

### 3.2 loadProfileSections() — Local Model Discovery

**File:** `src/tui/services/modelInitializationService.ts:264-325`

For each profile in OpenAI profiles:
- Calls `modelsListLocalOpenai(profile.baseUrl, profile.apiKey)` (NAPI)
- Maps returned model IDs to `NapiModelInfo` objects
- Sets `contextWindow: profile.contextWindow || 128000` (default 128k)
- Sets `maxOutput: profile.maxOutputTokens || 16384` (default 16k)

### 3.3 ModelSelection Interface — Where contextWindow Lives

**File:** `src/tui/types/provider.ts:45-75`

```typescript
interface ModelSelection {
  providerId: string;
  modelId: string;
  apiModelId: string;
  displayName: string;
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number;    // ← TypeScript knows this
  maxOutput: number;        // ← TypeScript knows this
  profileName?: string;
  profileConfig?: ProfileConfig;
}
```

### 3.4 ProfileConfig Interface

**File:** `src/utils/provider-config.ts:42-51`

```typescript
interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;      // ← Optional override
  maxOutputTokens?: number;    // ← Optional override
}
```

---

## 4. Compaction Threshold Deep Dive

### 4.1 The Threshold Calculation

**File:** `codelet/cli/src/compaction_threshold.rs:90-98`

```rust
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    let output_reservation = if output_reservation == 0 {
        SESSION_OUTPUT_TOKEN_MAX  // 32,000 fallback
    } else {
        output_reservation
    };
    context_window.saturating_sub(output_reservation)
}
```

**Algorithm:** `usable_context = context_window - min(max_output, 32k)`

Examples:
- Claude (200k, 8k output): 200,000 - 8,192 = **191,808**
- OpenAI (128k, 4k output): 128,000 - 4,096 = **123,904**
- Gemini (1M, 8k output): 1,000,000 - 8,192 = **991,808**
- High-output model (200k, 64k output): 200,000 - 32,000 = **168,000** (capped)

### 4.2 Where Threshold is Computed in stream_loop

**File:** `codelet/cli/src/interactive/stream_loop.rs:276-279`

```rust
let context_window = session.provider_manager().context_window() as u64;
let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
let threshold = calculate_usable_context(context_window, max_output_tokens);
```

### 4.3 CompactionHook — The Runtime Check

**File:** `codelet/core/src/compaction_hook.rs`

The `CompactionHook` is created with `(state, threshold)` and implements `StreamingPromptHook`:

- **`on_completion_call`** (before API call): Checks `state.total() > threshold` → cancels if exceeded
- **`check_compaction_with_payload`**: Uses `MAX(last_known_total, estimated_payload)` to catch large tool results not yet counted
- **`on_stream_completion_response_finish`** (after API call): Updates `TokenState` from API usage response

Token total calculation (CTX-002): `input + cache_read + cache_creation + output` (simple sum, no discounting).

### 4.4 Summarization Budget (Post-Compaction Target)

**File:** `codelet/cli/src/compaction_threshold.rs:58-64`

```rust
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {  // 50,000
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER
    }
}
```

Claude (200k): target after compaction = 150,000 tokens. Headroom = 191,808 - 150,000 = **41,808 tokens**.

---

## 5. NAPI Model Switch Call Chain

### 5.1 Complete Flow: TypeScript → Rust

```
selectModel(options)                          [modelSelectionService.ts]
  │
  ├─ if profileConfig → configureProfileEnvironment()
  │   └─ Sets: OPENAI_BASE_URL, OPENAI_API_KEY,
  │           OPENAI_CONTEXT_WINDOW, OPENAI_MAX_OUTPUT_TOKENS
  │
  ├─ BRANCH on model type:
  │   ├─ profileConfig exists → sessionSetModelProfile(sid, pid, mid)
  │   ├─ providerId === 'codex' → sessionSetModelProfile(sid, pid, mid)
  │   └─ else (cloud)          → sessionSetModel(sid, pid, mid)
  │
  └─ On success: persist to config, update Zustand store
```

### 5.2 NAPI → Rust: session_set_model()

**File:** `codelet/napi/src/session_manager.rs:6424-6454`

```
session_set_model(session_id, provider_id, model_id)
  ├─ SessionManager::instance().get_session()
  ├─ session.set_model()           ← metadata RwLocks
  ├─ model_string = "{provider_id}/{model_id}"
  └─ if provider_id == "codex":
       inner.provider_manager_mut().set_model_direct()
     else:
       inner.provider_manager_mut().select_model()
```

### 5.3 NAPI → Rust: session_set_model_profile()

**File:** `codelet/napi/src/session_manager.rs:6462-6483`

```
session_set_model_profile(session_id, provider_id, model_id)
  ├─ SessionManager::instance().get_session()
  ├─ session.set_model()           ← metadata RwLocks
  └─ inner.provider_manager_mut().set_model_direct()  ← ALWAYS
```

### 5.4 Two Layers of Model State in BackgroundSession

1. **Metadata layer:** `provider_id: RwLock<Option<String>>` + `model_id: RwLock<Option<String>>` — for display
2. **Engine layer:** `ProviderManager` inside `inner: Arc<Mutex<Session>>` — for actual API dispatch

Both NAPI functions update BOTH layers.

---

## 6. Key Findings & Gaps

### 6.1 CRITICAL: context_window() Ignores OPENAI_CONTEXT_WINDOW Env Var

**ProviderManager::context_window()** returns `openai::CONTEXT_WINDOW` (compile-time 128,000) regardless of `OPENAI_CONTEXT_WINDOW` env var.

Meanwhile:
- **OpenAIProvider::new()** reads `OPENAI_CONTEXT_WINDOW` and stores it in the instance
- **configureProfileEnvironment()** sets `OPENAI_CONTEXT_WINDOW` for profiles
- **ProviderManager::max_output_tokens()** DOES read `OPENAI_MAX_OUTPUT_TOKENS` at runtime (PROV-039)

**Impact:** For profile-based models (vLLM, Ollama), the compaction threshold is computed using 128k default instead of the configured context window. A local model with 32k context window would get threshold 123,904 (128k - 4k) instead of the correct ~28k.

### 6.2 Per-Model Context Window Not Used

The `ModelSelection.contextWindow` field is populated in TypeScript (from models.dev data or profile config) but is NEVER passed through the NAPI layer to Rust's ProviderManager. The Rust side always uses hardcoded per-provider constants.

### 6.3 Asymmetry: max_output_tokens has env override, context_window does not

`max_output_tokens()` was updated in PROV-039 to read `OPENAI_MAX_OUTPUT_TOKENS`. The same treatment was NOT applied to `context_window()` for `OPENAI_CONTEXT_WINDOW`.

### 6.4 All create_rig_agent() Use Constants (Except OpenAI)

Only `OpenAIProvider::create_rig_agent()` uses `self.max_output_tokens` (instance field from env var). All other providers use `MAX_OUTPUT_TOKENS` compile-time constants in `.max_tokens()`.

### 6.5 Facade Pattern is Well-Established

The facade dispatch chain is solid:
- 6 providers with per-provider tool facades
- `run_with_provider!` macro handles dispatch uniformly
- Each provider has its own `create_rig_agent()` with provider-specific tool naming
- `map_provider_id_to_type()` maps string IDs to enum variants

### 6.6 Model Switch is Decoupled from Agent Creation

Model switch (`session_set_model`/`session_set_model_profile`) only updates `ProviderManager` state. The actual provider instance and rig agent are created fresh on each `agent_loop` iteration via `run_with_provider!`.

---

*End of analysis*
