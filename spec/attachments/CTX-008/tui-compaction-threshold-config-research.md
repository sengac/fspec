# CTX-008: TUI Configuration Fields and NAPI Bridge — Research Document

## Current TUI Configuration Points for Context Window

### 1. Provider Settings Panel (Profile-Level)

**File:** `src/tui/constants/providerSettings.ts`

```typescript
export const PROFILE_FORM_FIELDS: Array<keyof ProfileConfig> = [
  'baseUrl',
  'apiKey',
  'contextWindow',      // ← existing
  'maxOutputTokens',    // ← existing
  // NEW: 'compactionThreshold' needed here
];
```

**File:** `src/tui/components/ProviderSettingsPanel.tsx`

Currently renders 4 fields: baseUrl, apiKey, contextWindow, maxOutputTokens.
Each field has a label, placeholder, and field-type-specific rendering.

### 2. Custom Model Form (Per-Model)

**File:** `src/tui/constants/customModelForm.ts`

```typescript
export const CUSTOM_MODEL_FORM_FIELDS: CustomModelFormField[] = [
  { key: 'id', ... },
  { key: 'displayName', ... },
  { key: 'facade', ... },
  { key: 'contextWindow', ... },      // ← existing (position 3)
  { key: 'maxOutputTokens', ... },    // ← existing (position 4)
  { key: 'reasoning', ... },
  { key: 'hasVision', ... },
  // NEW: 'compactionThreshold' needed here (between maxOutputTokens and reasoning)
];
```

### 3. Input Handling

**File:** `src/tui/inputHandlers/profileFormModeHandler.ts`

Numeric fields are parsed with `parseInt()`:

```typescript
if (field === 'contextWindow' || field === 'maxOutputTokens') {
  const num = parseInt(newValue, 10);
  return { ...prev, [field]: isNaN(num) ? undefined : num };
}
```

**New handling needed for compaction threshold:**
- Plain number (e.g., `200000`) → `{ type: 'tokens', value: 200000 }`
- Number with `%` suffix (e.g., `80%`) → `{ type: 'percentage', value: 80 }`
- Empty → `undefined` (use built-in default)

## Proposed Type Changes

### TypeScript Types

```typescript
// src/tui/types/provider.ts

export interface CompactionThresholdConfig {
  type: 'tokens' | 'percentage';
  value: number;
}

export interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
  compactionThreshold?: CompactionThresholdConfig;  // ← NEW
  customModels?: CustomModelDefinition[];
}

export interface CustomModelDefinition {
  id: string;
  displayName?: string;
  facade?: 'openai' | 'codex' | 'claude' | 'gemini' | 'zai';
  contextWindow?: number;
  maxOutputTokens?: number;
  compactionThreshold?: CompactionThresholdConfig;  // ← NEW
  reasoning?: boolean;
  hasVision?: boolean;
}

export interface ModelSelection {
  providerId: string;
  modelId: string;
  apiModelId: string;
  displayName: string;
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number;
  maxOutput: number;
  compactionThreshold?: CompactionThresholdConfig;  // ← NEW
  profileName?: string;
  profileConfig?: ProfileConfig;
  facade?: string;
}
```

### NAPI Bridge Types

```rust
// codelet/napi/src/types.rs

#[napi(object)]
pub struct NapiCompactionThreshold {
    /// "tokens" or "percentage"
    pub threshold_type: String,
    /// The value (token count or percentage 0-100)
    pub value: u32,
}
```

## NAPI Function Changes

### session_set_model

```rust
// codelet/napi/src/session_manager.rs

#[napi]
pub async fn session_set_model(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
    compaction_threshold: Option<NapiCompactionThreshold>,  // ← NEW
) -> Result<()>
```

### session_set_model_profile

```rust
#[napi]
pub async fn session_set_model_profile(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
    facade_override: Option<String>,
    compaction_threshold: Option<NapiCompactionThreshold>,  // ← NEW
) -> Result<()>
```

## Model Selection Service Changes

**File:** `src/tui/services/modelSelectionService.ts`

```typescript
// Profile-based model:
await sessionSetModelProfile(
  sessionId,
  selection.providerId,
  selection.modelId,
  selection.contextWindow,
  selection.maxOutput,
  selection.facade ?? null,
  selection.compactionThreshold ?? null,  // ← NEW
);

// Cloud model:
await sessionSetModel(
  sessionId,
  selection.providerId,
  selection.modelId,
  selection.contextWindow,
  selection.maxOutput,
  selection.compactionThreshold ?? null,  // ← NEW
);
```

## TUI Input UX Design

### Provider Settings Panel — Compaction Threshold Field

```
┌ Provider Settings ─────────────────────────┐
│ Base URL:           http://localhost:8888   │
│ API Key:            ••••••••               │
│ Context Window:     128000                 │
│ Max Output Tokens:  16384                  │
│ Compaction Trigger: 80%                    │  ← NEW
└────────────────────────────────────────────┘
```

**Label:** "Compaction Trigger" (user-friendly, avoids "threshold")
**Placeholder:** `80% or 200000`
**Help text:** "When to trigger compaction: percentage of context (e.g. 80%) or absolute token count"

### Custom Model Form — Compaction Threshold Field

```
┌ Add Custom Model ──────────────────────────┐
│ Model ID:           meta-llama/...         │
│ Display Name:       Llama 3.1 405B         │
│ Facade:             openai                 │
│ Context Window:     128000                 │
│ Max Output Tokens:  16384                  │
│ Compaction Trigger: 102400                 │  ← NEW
│ Reasoning:          false                  │
│ Vision:             false                  │
└────────────────────────────────────────────┘
```

### Input Parsing Logic

```typescript
function parseCompactionThreshold(input: string): CompactionThresholdConfig | undefined {
  const trimmed = input.trim();
  if (!trimmed) return undefined;
  
  if (trimmed.endsWith('%')) {
    const pct = parseInt(trimmed.slice(0, -1), 10);
    if (isNaN(pct) || pct < 1 || pct > 100) return undefined;
    return { type: 'percentage', value: pct };
  }
  
  const tokens = parseInt(trimmed, 10);
  if (isNaN(tokens) || tokens < 1000) return undefined;
  return { type: 'tokens', value: tokens };
}
```

## Config Persistence

The compaction threshold is saved to `fspec-config.json` along with other profile/custom model settings:

```json
{
  "providers": {
    "vllm-local": {
      "baseUrl": "http://localhost:8888",
      "apiKey": "...",
      "contextWindow": 128000,
      "maxOutputTokens": 16384,
      "compactionThreshold": { "type": "percentage", "value": 80 },
      "customModels": [
        {
          "id": "meta-llama/Meta-Llama-3.1-405B",
          "contextWindow": 128000,
          "compactionThreshold": { "type": "tokens", "value": 100000 }
        }
      ]
    }
  }
}
```

## Priority Resolution (Full Chain)

```
1. CustomModelDefinition.compactionThreshold  (per-model user config)
2. ProfileConfig.compactionThreshold          (profile-level user config)
3. NAPI parameter from ModelSelection         (passed to Rust)
4. Built-in model family defaults in Rust     (Claude=200k, etc.)
5. Legacy calculate_usable_context()          (fallback)
```

## Files to Modify

| File | Change |
|------|--------|
| `src/tui/types/provider.ts` | Add `CompactionThresholdConfig` interface, add field to `ProfileConfig`, `CustomModelDefinition`, `ModelSelection` |
| `src/tui/constants/providerSettings.ts` | Add `'compactionThreshold'` to `PROFILE_FORM_FIELDS` |
| `src/tui/constants/customModelForm.ts` | Add compactionThreshold field to `CUSTOM_MODEL_FORM_FIELDS` |
| `src/tui/inputHandlers/profileFormModeHandler.ts` | Add parsing for compaction threshold input (number or percentage) |
| `src/tui/components/ProviderSettingsPanel.tsx` | Render compaction threshold field |
| `src/tui/hooks/useCustomModelFormState.ts` | Include compactionThreshold in saved definition |
| `src/tui/hooks/useModelSelectorState.ts` | Copy compactionThreshold to ModelSelection on select |
| `src/tui/services/modelSelectionService.ts` | Pass compactionThreshold to NAPI calls |
| `codelet/napi/src/session_manager.rs` | Accept compaction_threshold param in set_model functions |
| `codelet/napi/src/types.rs` | Add `NapiCompactionThreshold` struct |
| `codelet/napi/index.d.ts` | Updated TypeScript type declarations |

## Test Strategy

1. **Input parsing:** `parseCompactionThreshold('80%')` → `{ type: 'percentage', value: 80 }`
2. **Input parsing:** `parseCompactionThreshold('200000')` → `{ type: 'tokens', value: 200000 }`
3. **Input parsing:** `parseCompactionThreshold('')` → `undefined`
4. **Config persistence:** compactionThreshold saved and restored from fspec-config.json
5. **NAPI bridge:** TypeScript CompactionThresholdConfig correctly maps to Rust NapiCompactionThreshold
6. **Model selection:** compactionThreshold flows from form → ModelSelection → NAPI → ProviderManager
7. **Priority chain:** Custom model threshold overrides profile threshold
8. **Default behavior:** Existing models without compactionThreshold set use legacy behavior
