# TUI-040: Agent Modal Model Selection - Implementation Plan

## Overview

Replace the provider-only selector in AgentModal with a unified provider/model selector that leverages MODEL-001's dynamic model registry from models.dev.

## Problem Statement

Currently, AgentModal only allows switching between providers (claude, openai, gemini, codex). Users cannot select specific models within a provider, limiting their ability to:
- Choose cost-effective models for simple tasks
- Select reasoning-capable models for complex problems
- Use vision-enabled models for image analysis
- Optimize for context window size

## Solution

Implement a hierarchical model selector that:
1. Groups models by provider
2. Shows model capabilities at a glance
3. Filters to only tool_call-capable models
4. Uses existing MODEL-001 NAPI bindings

---

## Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  1. UI Layer (AgentModal.tsx)                               │
│     - Tab key trigger                                        │
│     - Hierarchical model selector overlay                    │
│     - Header display with capability indicators              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  2. State Layer (React)                                      │
│     - currentModel: ModelSelection                           │
│     - providerSections: ProviderSection[]                    │
│     - showModelSelector, expandedProviders                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  3. NAPI Layer (codelet-napi)                               │
│     - modelsListAll() → Vec<NapiProviderModels>             │
│     - CodeletSession.newWithModel(modelString)              │
│     - CodeletSession.selectModel(modelString)               │
│     - CodeletSession.selectedModel getter                    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  4. Provider Layer (codelet-providers)                       │
│     - ModelRegistry: validation, lookup                      │
│     - ProviderManager.select_model()                         │
│     - ProviderManager.selected_model_string()                │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  5. Cache Layer                                              │
│     - ~/.fspec/cache/models.json                             │
│     - Embedded fallback in binary                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Selector Open

```
1. User presses Tab key in AgentModal
       │
       ▼
2. modelsListAll() called to get models grouped by provider
       │
       ▼
3. Filter providers to those with credentials (availableProviders intersection)
       │
       ▼
4. Filter models to those with tool_call=true capability
       │
       ▼
5. Display hierarchical selector with current provider expanded
```

### Model Selection

```
1. User navigates with arrow keys, Enter to select
       │
       ▼
2. selectModel("provider/model-id") called on CodeletSession
       │
       ▼
3. ProviderManager.select_model() validates and switches model
       │
       ▼
4. Header updated to show model name and capability indicators
```

---

## File Structure

| File | Purpose |
|------|---------|
| `src/tui/components/AgentModal.tsx` | Main modal component, selector overlay |
| `codelet/napi/src/session.rs` | CodeletSession with newWithModel(), selectModel() |
| `codelet/napi/src/models.rs` | modelsListAll(), modelsListForProvider(), modelsGetInfo() |
| `codelet/providers/src/manager.rs` | ProviderManager.select_model(), selected_model_string() |
| `codelet/providers/src/models/registry.rs` | ModelRegistry validation |

---

## TypeScript Types

```typescript
// New types to add to AgentModal.tsx

interface ModelSelection {
  providerId: string;      // "anthropic"
  modelId: string;         // "claude-sonnet-4"
  apiModelId: string;      // "claude-sonnet-4-20250514" (for API calls)
  displayName: string;     // "Claude Sonnet 4"
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number;   // 200000
  maxOutput: number;       // 16000
}

interface ProviderSection {
  providerId: string;      // "anthropic"
  providerName: string;    // "Anthropic"
  models: NapiModelInfo[]; // Filtered to tool_call=true
  hasCredentials: boolean; // From availableProviders check
}

// From codelet-napi (already exists in models.rs)
interface NapiModelInfo {
  id: string;              // API model ID
  name: string;            // Display name
  family?: string;
  reasoning: boolean;
  toolCall: boolean;
  attachment: boolean;
  temperature: boolean;
  contextWindow: number;
  maxOutput: number;
  hasVision: boolean;
}
```

---

## State Changes

### Current State (to be replaced)

```typescript
const [currentProvider, setCurrentProvider] = useState<string>('');
const [availableProviders, setAvailableProviders] = useState<string[]>([]);
const [showProviderSelector, setShowProviderSelector] = useState(false);
const [selectedProviderIndex, setSelectedProviderIndex] = useState(0);
```

### New State

```typescript
// Model selection state
const [currentModel, setCurrentModel] = useState<ModelSelection | null>(null);
const [providerSections, setProviderSections] = useState<ProviderSection[]>([]);
const [showModelSelector, setShowModelSelector] = useState(false);

// Navigation state for hierarchical selector
const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
const [selectedModelIdx, setSelectedModelIdx] = useState(-1); // -1 = on section header
const [expandedProviders, setExpandedProviders] = useState<Set<string>>(new Set());
```

---

## Provider ID Mapping

The models.dev API uses different provider IDs than our internal provider names:

| models.dev ID | Internal Name |
|---------------|---------------|
| `anthropic` | `claude` |
| `google` | `gemini` |
| `openai` | `openai` |

```typescript
const mapProviderIdToName = (providerId: string): string => {
  switch (providerId) {
    case 'anthropic': return 'claude';
    case 'google': return 'gemini';
    default: return providerId;
  }
};

const mapProviderNameToId = (providerName: string): string => {
  switch (providerName) {
    case 'claude': return 'anthropic';
    case 'gemini': return 'google';
    default: return providerName;
  }
};
```

---

## Critical Implementation Requirements

1. **MUST use `CodeletSession.newWithModel()`** for session creation (not basic constructor)
2. **MUST filter to only models with `tool_call=true`** capability
3. **MUST show capability indicators** (`[R]` reasoning, `[V]` vision, `[200k]` context)
4. **MUST persist full model path** `"provider/model-id"` in session manifest
5. **MUST support mid-session model switching** via `selectModel()`
6. **MUST gracefully handle missing models** with fallback to provider default

---

## UI Mockup

### Model Selector Overlay

```
╔══════════════════════════════════════════════════════════════╗
║                       Select Model                            ║
╠══════════════════════════════════════════════════════════════╣
║  ▼ [anthropic] (3 models)                                     ║
║    > claude-sonnet-4 (Claude Sonnet 4) [R] [200k] (current)   ║
║      claude-opus-4 (Claude Opus 4) [R] [200k]                 ║
║      claude-haiku (Claude Haiku) [200k]                       ║
║  ▶ [google] (2 models)                                        ║
║  ▶ [openai] (4 models)                                        ║
╠══════════════════════════════════════════════════════════════╣
║  Enter Select | ←→ Expand/Collapse | ↑↓ Navigate | Esc Cancel ║
╚══════════════════════════════════════════════════════════════╝
```

### Header Display

```
╔══════════════════════════════════════════════════════════════════════════════╗
║ Agent: claude-sonnet-4 [R] [200k]              tokens: 1234↓ 567↑ [12%] [Tab]║
╠══════════════════════════════════════════════════════════════════════════════╣
```

---

## Keyboard Navigation

| Key | Action |
|-----|--------|
| Tab | Open model selector |
| ↑/↓ | Navigate models/providers |
| ←   | Collapse provider section |
| →   | Expand provider section |
| Enter | Select model (or expand if on header) |
| Esc | Cancel and close |

---

## Implementation Phases

### Phase 1: NAPI Type Generation

Regenerate TypeScript definitions to include MODEL-001 bindings:

```bash
cd codelet/napi
npm run build
```

Verify these exports exist in `index.d.ts`:
- `modelsListAll(): Promise<NapiProviderModels[]>`
- `CodeletSession.newWithModel(modelString: string): Promise<CodeletSession>`
- `CodeletSession.selectModel(modelString: string): Promise<void>`
- `CodeletSession.selectedModel: string | null`

### Phase 2: Session Initialization

Update `initSession` in AgentModal.tsx to use `newWithModel()`:

```typescript
const { modelsListAll, CodeletSession } = codeletNapi;

// Load available models
const allModels = await modelsListAll();
const availableProviderNames = new CodeletSession().availableProviders;

// Filter and build sections
const sections = buildProviderSections(allModels, availableProviderNames);

// Find default model
const defaultModel = findFirstAvailableModel(sections);
const modelString = `${defaultModel.providerId}/${defaultModel.modelId}`;

// Create session with model
const newSession = await CodeletSession.newWithModel(modelString);
```

### Phase 3: Model Selector Component

Create the selector overlay component with:
- Collapsible provider sections
- Model list with capability indicators
- Current model highlighting
- Keyboard navigation

### Phase 4: Header Display Update

Update header to show model info:

```typescript
<Text bold color="cyan">
  Agent: {currentModel?.modelId || 'loading...'}
</Text>
{currentModel?.reasoning && <Text color="magenta"> [R]</Text>}
{currentModel?.hasVision && <Text color="blue"> [V]</Text>}
<Text dimColor> [{formatContextWindow(currentModel?.contextWindow)}]</Text>
```

### Phase 5: Persistence Integration

Update session creation and resume to use full model path:

```typescript
// Create session
persistenceCreateSessionWithProvider(
  name, project,
  `${currentModel.providerId}/${currentModel.modelId}`
);

// Resume session
if (provider.includes('/')) {
  await session.selectModel(provider);
} else {
  // Legacy: provider-only format
  await session.switchProvider(provider);
}
```

---

## Testing Plan

### Scenarios to Test

| Category | Scenario |
|----------|----------|
| Basic | Tab opens selector, arrow navigation works, Enter selects |
| Capability | [R], [V], context size indicators display correctly |
| Filtering | Only credentialed providers shown, only tool_call models |
| Session | newWithModel used, model path persisted, resume restores |
| Error | Missing model falls back, cache failure uses fallback |
| Legacy | Provider-only sessions use default model |

---

## Backward Compatibility

- Old sessions with provider-only storage (e.g., "claude") still work
- Legacy sessions use default model for that provider
- API key availability still gates provider access
- Graceful fallback if model selection fails

---

## Dependencies

- MODEL-001 implementation (complete)
- NAPI bindings regeneration (required before implementation)
- No external dependencies
