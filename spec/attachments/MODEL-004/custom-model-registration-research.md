# MODEL-004: Custom Model Registration and Facade Override — Research

## Problem Statement

Users with OpenAI-compatible API endpoints cannot add models that aren't listed in the `/v1/models` endpoint. This affects three common scenarios:

1. **Private/fine-tuned models** — The model ID exists on the server but isn't returned by `/v1/models` (some proxies filter results).
2. **Proxy/gateway servers** — Services like LiteLLM or custom API gateways may not implement `/models` at all, returning 404 or an empty list.
3. **Unreleased or preview models** — A model behind a beta flag that the server knows about but doesn't advertise.
4. **Facade mismatch** — A model is listed but the system auto-assigns the wrong facade (e.g., a Codex-compatible model routed through the generic OpenAI facade).

Currently, the **only** way models appear in the selector for local profiles is via the `/v1/models` endpoint call. If that call returns nothing, the profile section shows zero models and is unusable.

---

## Current Architecture

### How Local Profile Models Are Discovered

**File:** `src/tui/services/modelInitializationService.ts` → `loadProfileSections()` (lines 264–325)

For each profile in `~/.fspec/fspec-config.json` → `providers.openai.profiles`:

```
loadProfileSections()
  → loadProviderProfiles('openai')
  → for each profile:
      modelsListLocalOpenai(profile.baseUrl, profile.apiKey)  ← NAPI call
      → GET {baseUrl}/v1/models
      → returns string[] of model IDs
```

The NAPI function `modelsListLocalOpenai` lives in `codelet/napi/src/models/napi_bindings.rs` and hits the standard OpenAI-compatible `/v1/models` endpoint. Each returned model ID is wrapped into a synthetic `NapiModelInfo` with hardcoded defaults:

```typescript
// modelInitializationService.ts ~line 290
{
  id: modelId,
  name: modelId,
  reasoning: false,
  toolCall: true,
  attachment: false,
  temperature: true,
  contextWindow: profile.contextWindow || 128000,
  maxOutput: profile.maxOutputTokens || 16384,
  hasVision: false,
}
```

**Key insight:** There is no mechanism to manually inject models into this list. If `/v1/models` fails or returns an empty list, the profile appears with `isUnreachable: true` and no models.

### How Facade Type Is Determined

**File:** `codelet/napi/src/session_manager.rs` (lines 3273–3348, 5165–5255)

The facade is determined entirely by the **provider ID** string, not by the model. The dispatch chain:

```
model string "openai:my-profile/my-model"
  → is_profile_model = true
  → provider_manager.set_model_direct("openai", "my-model")
  → current_provider = ProviderType::OpenAI
  → agent loop matches "openai" → get_openai()
  → OpenAI tool facades (standard OpenAI schema)
```

**Problem:** ALL local profile models get the `OpenAI` facade. There's no way to say "this model on my vLLM server speaks the Codex tool schema" or "this model works best with Gemini-style tool formatting."

### Provider-to-Facade Mapping

| ProviderType | Facade Set | Tool Schema |
|---|---|---|
| `Claude` | `select_claude_facade()` | Claude-native (PascalCase: `Read`, `Write`, `Bash`) |
| `OpenAI` | `OpenAISystemPromptFacade` | Standard OpenAI function calling |
| `Codex` | `CodexShellFacade`, `CodexReadFileFacade`, etc. | Codex-native (`exec_command`, `shell`, `read_file`, `grep_files`, `list_dir`) |
| `Gemini` | `GeminiReadFileFacade`, etc. | Gemini-native snake_case |
| `ZAI` | `ZAIReadFileFacade`, etc. | ZAI/GLM-native |
| `GitHubCopilot` | Behavior facade dispatches by model family | Varies (GPT/Claude/Gemini styles) |

### Where Models Are Stored/Configured

**Cloud models:** `~/.codelet/cache/models.json` (fetched from models.dev, managed by `ModelCache`/`ModelRegistry` in Rust)

**Profile definitions:** `~/.fspec/fspec-config.json` → `providers.openai.profiles`
```json
{
  "providers": {
    "openai": {
      "profiles": {
        "work-vllm": {
          "baseUrl": "http://localhost:8888",
          "apiKey": "sk-...",
          "contextWindow": 131072,
          "maxOutputTokens": 16384
        }
      }
    }
  }
}
```

**Selected model persistence:** `~/.fspec/fspec-config.json` → `tui.lastUsedModel`
- Cloud format: `"anthropic/claude-sonnet-4"`
- Profile format: `"openai:work-vllm/Qwen/Qwen3-80B"`

---

## Proposed Solution Direction

### 1. Custom Model Configuration in ProfileConfig

Extend `ProfileConfig` (or add a sibling structure) to allow manually defined models:

```typescript
interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
  // NEW
  customModels?: CustomModelDefinition[];
}

interface CustomModelDefinition {
  id: string;                    // Model ID string to send to the API
  displayName?: string;          // Optional human-friendly name
  facade?: 'openai' | 'codex' | 'claude' | 'gemini' | 'zai';  // Override facade
  contextWindow?: number;        // Per-model override (takes precedence over profile-level)
  maxOutputTokens?: number;      // Per-model override
  reasoning?: boolean;           // Whether this model supports reasoning
  hasVision?: boolean;           // Whether this model supports image input
}
```

### 2. TUI Flow for Adding Custom Models

In the Model Selector screen, when focused on a profile section:
- New keybind (e.g., `a` for "add model") opens a form to add a custom model
- Form fields: Model ID (required), Display Name, Facade Type (dropdown), Context Window, Max Output
- New keybind (e.g., `e` for "edit") on an existing model to change its facade/context settings
- New keybind (e.g., `d` for "delete") on a custom model to remove it

### 3. Facade Override Dispatch

When a model with a `facade` override is selected, the dispatch in `session_manager.rs` needs a secondary lookup:

```
Current: provider_type → facade set
Proposed: model_config.facade || provider_type → facade set
```

This requires propagating the facade override from the TypeScript `ModelSelection` through the NAPI boundary to the Rust `ProviderManager`, likely as a new field on the session or model config.

### 4. Cloud Model Override

For cloud models (from models.dev), allow override of facade and context settings. This could use a separate config section:

```json
{
  "modelOverrides": {
    "openai/o3-pro": {
      "facade": "codex",
      "contextWindow": 200000
    }
  }
}
```

---

## Key Files to Modify

| Layer | File | Changes |
|---|---|---|
| Config | `src/utils/provider-config.ts` | Add `CustomModelDefinition` interface, extend `ProfileConfig` |
| NAPI | `codelet/napi/src/models/napi_bindings.rs` | Add NAPI struct for custom model config |
| NAPI | `codelet/napi/src/session_manager.rs` | Facade override dispatch logic |
| Providers | `codelet/providers/src/manager.rs` | Accept facade override in `set_model_direct()` |
| TUI Service | `src/tui/services/modelInitializationService.ts` | Merge custom models into profile sections |
| TUI Hook | `src/tui/hooks/useModelSelectorState.ts` | Add/edit/delete model actions |
| TUI Screen | `src/tui/screens/ModelSelectorScreen.tsx` | Keyboard handlers for a/e/d keybinds |
| TUI View | `src/tui/views/ModelSelectorView.tsx` | Render custom model badge/indicator |
| TUI Types | `src/tui/types/provider.ts` | Extend `ModelSelection` with facade override |
| Persistence | `src/tui/services/modelSelectionService.ts` | Persist facade override in model string |

---

## Risks and Open Questions

1. **Facade-provider mismatch at API level:** If a user assigns the Codex facade to a vLLM server, the server might not understand Codex-specific parameters. Need clear documentation or warnings.
2. **Thinking config coupling:** Facades like Claude have deep thinking config integration (`ClaudeThinkingFacade`). A custom model with `facade: 'claude'` would need to handle thinking config gracefully even if the model doesn't support it.
3. **Should cloud model overrides be per-profile or global?** A user might want to override `openai/gpt-4o` globally, or only when accessed through a specific proxy profile.
4. **Config migration:** Existing `ProfileConfig` entries need to remain backward-compatible.
5. **Validation:** When adding a custom model, should we validate the model ID against the server, or trust the user's input entirely?
