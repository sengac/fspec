# MODEL-005: Per-Model Context Window and Max Output Configuration — Research

## Problem Statement

The context window and max output tokens are currently **hardcoded as compile-time constants per provider type**. The compaction engine, which decides when to compress conversation history, uses these provider-level constants. However, models within the same provider can have wildly different context windows:

| Provider | Provider Constant | Actual Model Range |
|---|---|---|
| **Anthropic** | 200,000 | Claude Haiku = 200k, Claude Opus = 200k (but 1M beta exists) |
| **OpenAI** | 128,000 | GPT-4o = 128k, GPT-4o-mini = 128k, o3 = 200k, o3-pro = 200k |
| **Gemini** | 1,000,000 | Gemini 2.5 Pro = 1M, Gemini 2.5 Flash = 1M, Gemini 2.0 Flash = 1M |
| **Codex** | 272,000 | codex-mini = 272k |
| **Z.AI** | 128,000 | GLM-4-Plus = 128k |
| **Copilot** | 200,000 | Routes to GPT/Claude/Gemini models with different actual limits |

**The problem manifests in three ways:**

1. **Over-compaction:** A model with a larger context than the provider constant (e.g., o3 at 200k on OpenAI's 128k constant) triggers compaction too early, throwing away useful context.
2. **Under-compaction:** A model with a smaller context than the provider constant (e.g., a fine-tuned 32k model on a vLLM profile using the 128k OpenAI default) never triggers compaction until the API rejects the request as too large.
3. **Display vs. reality gap:** The TUI model selector correctly shows `[200k]` from models.dev metadata per-model, but the compaction engine ignores this and uses the provider constant. Users see the right number but get the wrong behavior.

---

## Current Architecture: Two Disconnected Context Window Systems

### System 1: Provider-Level Constants (Rust — used by compaction engine)

**File:** `codelet/providers/src/manager.rs` (lines 622–653)

```rust
pub fn context_window(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::CONTEXT_WINDOW,        // 200,000
        ProviderType::OpenAI => {
            // Runtime override via env var
            std::env::var("OPENAI_CONTEXT_WINDOW")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(openai::DEFAULT_CONTEXT_WINDOW)     // 128,000
        }
        ProviderType::Codex => codex::CONTEXT_WINDOW,          // 272,000
        ProviderType::Gemini => gemini::CONTEXT_WINDOW,        // 1,000,000
        ProviderType::ZAI => zai::CONTEXT_WINDOW,              // 128,000
        ProviderType::GitHubCopilot => copilot::CONTEXT_WINDOW,// 200,000
    }
}

pub fn max_output_tokens(&self) -> usize {
    match self.current_provider {
        ProviderType::Claude => claude::MAX_OUTPUT_TOKENS,     // 8,192
        ProviderType::OpenAI => {
            std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(openai::DEFAULT_MAX_OUTPUT_TOKENS)  // 16,384
        }
        ProviderType::Codex => codex::MAX_OUTPUT_TOKENS,       // 4,096
        ProviderType::Gemini => gemini::MAX_OUTPUT_TOKENS,     // 8,192
        ProviderType::ZAI => zai::MAX_OUTPUT_TOKENS,           // 8,192
        ProviderType::GitHubCopilot => copilot::MAX_OUTPUT_TOKENS, // 4,096
    }
}
```

**Consumers of `context_window()`:**
- **Compaction engine** — decides when to trigger context compaction (the primary consumer)
- **Token counting** — calculates fill percentage for the context window indicator
- **Agent loop** — uses max_output_tokens for API request parameters

### System 2: Per-Model Metadata (models.dev → TUI — displayed but not used for compaction)

**File:** `codelet/napi/src/models/napi_bindings.rs`

```rust
pub struct NapiModelInfo {
    // ...
    pub context_window: u32,   // From models.dev LimitInfo.context
    pub max_output: u32,       // From models.dev LimitInfo.output
}
```

**File:** `src/tui/types/provider.ts`

```typescript
interface ModelSelection {
    // ...
    contextWindow: number;     // Stored when model is selected
    maxOutput: number;         // Stored when model is selected
}
```

This data flows: `models.dev API → ModelCache → ModelRegistry → NapiModelInfo → ProviderSection → ModelSelection → Zustand modelStore`

**But it stops there.** The `ModelSelection.contextWindow` is never passed to the Rust `ProviderManager`. The Rust side has no knowledge of the per-model metadata — it only knows the provider type.

### The OpenAI Environment Variable Workaround

For local profiles (vLLM/Ollama), there's a partial workaround:

**File:** `src/tui/services/profileEnvironmentService.ts`

```typescript
export function configureProfileEnvironment(config: ProfileConfig): void {
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

This sets environment variables that the Rust `OpenAIProvider` reads. **But this only works for the OpenAI provider** — no other provider has runtime context window override support. And it's set at the **profile** level, not the model level, so all models within a profile share the same context window.

### Copilot's Compound Problem

The Copilot provider is a **proxy** for multiple model families (GPT, Claude, Gemini). It uses a single hardcoded constant (`CONTEXT_WINDOW = 200,000`), but the underlying models have very different limits. A Copilot session using `gemini-2.5-pro` (which has 1M context) is artificially limited to 200k compaction.

---

## Data Flow Diagram (Current)

```
models.dev API
    │
    ▼
ModelRegistry (Rust)
    │  per-model: context=200k, output=8k
    ▼
NapiModelInfo (NAPI boundary)
    │  context_window: 200000
    ▼
ModelSelection (TypeScript/Zustand)
    │  contextWindow: 200000
    │
    │  ╳ DEAD END — never sent back to Rust
    │
    ▼
TUI displays "[200k]"        ProviderManager (Rust)
                                  │  context_window() → match provider {
                                  │      Claude => 200_000  ← compile-time constant
                                  │  }
                                  ▼
                              Compaction engine uses 200k
```

---

## Proposed Solution Direction

### 1. Add Per-Model Context Config to ProviderManager

The `ProviderManager` needs a `model_context_window` and `model_max_output_tokens` field that gets set when a model is selected:

```rust
pub struct ProviderManager {
    // ... existing fields
    model_context_window: Option<usize>,     // NEW: from models.dev or user override
    model_max_output_tokens: Option<usize>,  // NEW: from models.dev or user override
}

impl ProviderManager {
    pub fn context_window(&self) -> usize {
        // Priority: model-specific > env var > provider constant
        self.model_context_window
            .unwrap_or_else(|| self.provider_constant_context_window())
    }
}
```

### 2. Propagate Model Metadata Through NAPI

When `sessionSetModel` or `sessionSetModelProfile` is called, also pass the model's context window and max output:

```rust
#[napi]
fn session_set_model_with_context(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
) -> Result<()>
```

### 3. Resolution Priority Chain

```
1. User override (from customModels config or model override) — highest
2. models.dev per-model metadata (from ModelRegistry)
3. Environment variable (OPENAI_CONTEXT_WINDOW etc.)
4. Provider-level compile-time constant — lowest (fallback)
```

### 4. Remove Provider-Level Constants (Eventually)

The compile-time constants become fallback-only defaults for when no model-specific data is available. They should no longer be the primary source for any model that has metadata.

---

## Key Files to Modify

| Layer | File | Changes |
|---|---|---|
| Providers | `codelet/providers/src/manager.rs` | Add `model_context_window`/`model_max_output_tokens` fields, update `context_window()`/`max_output_tokens()` |
| Providers | `codelet/providers/src/manager.rs` | Update `select_model()` and `set_model_direct()` to accept and store model-level limits |
| NAPI | `codelet/napi/src/session_manager.rs` | Extend `session_set_model`/`session_set_model_profile` to pass context window and max output |
| NAPI | `codelet/napi/src/models/napi_bindings.rs` | Ensure `NapiModelInfo.context_window` and `max_output` are correctly populated from registry |
| TUI Service | `src/tui/services/modelSelectionService.ts` | Pass `ModelSelection.contextWindow` and `maxOutput` through to NAPI calls |
| TUI Service | `src/tui/services/profileEnvironmentService.ts` | May be simplified — env var approach could be replaced by direct NAPI parameter passing |
| Compaction | Compaction engine (Rust) | Verify it reads from `ProviderManager.context_window()` (it already does, but verify chain) |
| Provider Constants | `codelet/providers/src/claude.rs`, `openai.rs`, etc. | Keep constants as fallbacks, document they're no longer primary |

### Provider Constant Locations

| File | Constants |
|---|---|
| `codelet/providers/src/claude.rs` | `CONTEXT_WINDOW = 200_000`, `MAX_OUTPUT_TOKENS = 8_192` |
| `codelet/providers/src/openai.rs` | `DEFAULT_CONTEXT_WINDOW = 128_000`, `DEFAULT_MAX_OUTPUT_TOKENS = 16_384` |
| `codelet/providers/src/codex.rs` | `CONTEXT_WINDOW = 272_000`, `MAX_OUTPUT_TOKENS = 4_096` |
| `codelet/providers/src/gemini.rs` | `CONTEXT_WINDOW = 1_000_000`, `MAX_OUTPUT_TOKENS = 8_192` |
| `codelet/providers/src/zai.rs` | `CONTEXT_WINDOW = 128_000`, `MAX_OUTPUT_TOKENS = 8_192` |
| `codelet/providers/src/copilot/constants.rs` | `CONTEXT_WINDOW = 200_000`, `MAX_OUTPUT_TOKENS = 4_096` |

---

## Impact Analysis

### What Changes for Each Provider

| Provider | Current Behavior | After Fix |
|---|---|---|
| **Anthropic** | All models use 200k constant | Each model uses its models.dev value (most are 200k, but future 1M models get 1M) |
| **OpenAI** | All models use 128k (or env var) | Per-model from models.dev: o3=200k, gpt-4o=128k, gpt-4o-mini=128k |
| **Codex** | All models use 272k constant | Per-model from registry |
| **Gemini** | All models use 1M constant | Per-model (currently all are 1M, future smaller models would be correct) |
| **Copilot** | All models use 200k constant | Per-model: GPT models get GPT limits, Claude models get Claude limits |
| **Local profiles** | All models in profile share profile-level setting | Per-model override possible via `customModels` config |

### What Breaks

- **Environment variable overrides** (`OPENAI_CONTEXT_WINDOW`) should still work but at lower priority than models.dev metadata
- **Compaction timing changes** — some models will compact earlier or later than before. This is the desired behavior but could surprise users
- **Tests** that mock `context_window()` return values may need updating

---

## Relationship to MODEL-004

MODEL-004 (Custom Model Registration) introduces `CustomModelDefinition.contextWindow` and `CustomModelDefinition.maxOutputTokens` — these are user-level overrides that feed into the same resolution chain proposed here. MODEL-005 creates the infrastructure that MODEL-004's per-model overrides plug into.

**Dependency direction:** MODEL-005 should be implemented first (or concurrently) since it establishes the per-model context resolution infrastructure. MODEL-004's custom model overrides then naturally feed into this system at the highest priority level.

---

## Risks

1. **Compaction behavior change:** Users may notice different compaction timing. Models that were over-compacting will retain more context; models that were under-compacting may hit API limits during the transition.
2. **models.dev data quality:** If models.dev returns incorrect context window values, the system will use wrong values. Fallback to provider constants provides a safety net.
3. **Copilot complexity:** Copilot's multi-family routing means the context window must be resolved after the model family is determined, not just from the `ProviderType`.
4. **CONFIG-007 interaction:** The existing CONFIG-007 card ("Add 1M context window opt-in for Anthropic Tier 4 users") would be partially resolved by this work — if models.dev reports 1M for eligible models, the system would automatically use it.
