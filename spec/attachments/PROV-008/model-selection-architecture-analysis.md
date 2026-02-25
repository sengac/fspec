# Model Selection Architecture Analysis

**Work Unit:** PROV-008  
**Date:** 2026-02-25  
**Status:** Analysis Complete

---

## Executive Summary

The model selection system spanning TypeScript (TUI) and Rust (codelet-providers) layers exhibits multiple code smells that violate DRY, SOLID principles, and separation of concerns. This document details the findings and proposes targeted fixes.

---

## 1. Root Cause: The Warning Noise

### Symptom
```
[RUST:WARN] parse_model_string: provider 'Qwen' not in registry
```
This warning appears repeatedly when using profile-based models (vLLM, Ollama, etc.).

### Cause
The `ProviderManager::selected_model_id()` method in `codelet/providers/src/manager.rs` (lines 297-312) always attempts registry lookup, even for profile-based models:

```rust
pub fn selected_model_id(&self) -> Option<String> {
    let model_string = self.selected_model.as_ref()?;

    // PROBLEM: Always tries registry lookup, even for profile models
    if let Some(registry) = self.model_registry.as_ref() {
        if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
            // This triggers the warning because "Qwen/..." is parsed as provider=Qwen
            ...
        }
    }
    // Falls through to return the string directly (correct for profiles)
    Some(model_string.clone())
}
```

When `set_model_direct` stores `"Qwen/Qwen3-Next-80B-A3B-Instruct-FP8"`, subsequent calls try to parse it as `provider/model-id`, treating `"Qwen"` as a cloud provider name.

---

## 2. DRY Violations

### 2.1 TypeScript: Duplicate Model Selection Handlers

**Location:** `src/tui/components/AgentView.tsx` lines 3615-3749

Two nearly identical functions exist:
- `handleModelSelect` (lines 3615-3689) - Takes `ModelSelection`
- `handleSelectModel` (lines 3693-3749) - Takes `ProviderSection` + `NapiModelInfo` (marked deprecated)

**Duplicated Logic:**
1. Environment variable setup for profile configs
2. Calling `sessionSetModel` or `sessionSetModelProfile`
3. Refreshing Rust state
4. Persisting model selection to user config
5. Setting local state when no session exists

**Fix Required:** Delete `handleSelectModel`, update all callers to use `handleModelSelect`.

### 2.2 Rust: Repeated Provider Mapping

**Location:** `codelet/providers/src/manager.rs`

The provider ID to ProviderType mapping appears in:
- `map_provider_id_to_type()` (lines 340-354)
- `ProviderType::from_str()` (lines 32-44)
- `detect_default_provider()` (lines 357-381)

**Fix Required:** Create a single `ProviderType::from_models_dev_id()` method.

---

## 3. Single Responsibility Principle (SRP) Violations

### 3.1 `ModelRegistry::parse_model_string` Does Too Much

**Location:** `codelet/providers/src/models/registry.rs` lines 52-79

Current responsibilities:
1. Parses the model string (splitting on `/`)
2. Validates provider exists in registry
3. Logs debug warnings
4. Returns detailed error with suggestions

**Fix Required:** Split into:
```rust
// Pure parsing - no validation, no logging
pub fn parse_model_string(input: &str) -> Option<(String, String)>

// Separate validation
pub fn validate_provider(&self, provider_id: &str) -> Result<(), ProviderError>
```

### 3.2 `ProviderManager::selected_model_id` Has Implicit Branching

**Location:** `codelet/providers/src/manager.rs` lines 297-312

The method has two behaviors based on implicit conditions:
1. Registry-validated lookup (for cloud models)
2. Direct string return (for profile models)

**Fix Required:** Make this explicit with a discriminated union type or explicit method variants.

---

## 4. Open/Closed Principle Violations

### 4.1 Hardcoded Provider Mapping

**Location:** `codelet/providers/src/manager.rs` lines 340-354

```rust
fn map_provider_id_to_type(provider_id: &str) -> Result<ProviderType, ProviderError> {
    match provider_id {
        "anthropic" => Ok(ProviderType::Claude),
        "openai" => Ok(ProviderType::OpenAI),
        "google" => Ok(ProviderType::Gemini),
        "zai" | "z-ai" => Ok(ProviderType::ZAI),
        _ => Err(...),
    }
}
```

Adding a new provider requires modifying this function.

**Fix Required:** Use a registry pattern or trait-based dispatch.

---

## 5. Separation of Concerns Issues

### 5.1 Model State Scattered Across 5 Locations

| Location | Type | What it stores |
|----------|------|----------------|
| `ProviderManager.selected_model` | Rust | Model string |
| `BackgroundSession.provider_id` | Rust | Provider ID |
| `BackgroundSession.model_id` | Rust | Model ID |
| `modelStore.currentModel` | TypeScript/Zustand | Full `ModelSelection` object |
| User config `lastUsedModel` | JSON file | Persisted model string |

**Problems:**
- State can become inconsistent between layers
- No single source of truth
- Debugging requires checking multiple locations

**Fix Required:** 
1. Define Rust as the authoritative source
2. TypeScript reads from Rust, never caches independently
3. Persistence happens once, in one place

### 5.2 Environment Variable Setup in Component

**Location:** `src/tui/components/AgentView.tsx` lines 3626-3641

```typescript
if (selection.profileConfig) {
  process.env.OPENAI_BASE_URL = selection.profileConfig.baseUrl;
  process.env.OPENAI_API_KEY = selection.profileConfig.apiKey;
  // ...
}
```

Environment variable setup is a side effect buried in a UI component.

**Fix Required:** Extract to a dedicated service:
```typescript
// src/tui/services/profileEnvironmentService.ts
export function configureProfileEnvironment(config: ProfileConfig): void
```

---

## 6. Missing Type Discrimination

### 6.1 Cloud vs Profile Models Use Same Type

There's no way to distinguish between:
- Cloud model strings: `"anthropic/claude-sonnet-4"`
- Profile model IDs: `"Qwen/Qwen3-Next-80B-A3B-Instruct-FP8"`

Both are stored as `String` in `ProviderManager.selected_model`.

**Fix Required:** Introduce a discriminated union:

```rust
pub enum SelectedModel {
    /// Cloud provider model (validated via registry)
    Cloud { provider_id: String, model_id: String },
    /// Profile-based model (direct API model ID, no registry validation)
    Profile { provider_type: ProviderType, model_id: String },
}
```

---

## 7. Proposed Fixes

### 7.1 Quick Fix: Suppress Warning for Profile Models

**File:** `codelet/providers/src/manager.rs`

```rust
pub fn selected_model_id(&self) -> Option<String> {
    let model_string = self.selected_model.as_ref()?;

    // Only attempt registry lookup for recognized cloud providers
    if let Some(registry) = self.model_registry.as_ref() {
        if let Some((provider, _)) = model_string.split_once('/') {
            // Check if provider is in registry before parsing
            if registry.list_provider_ids().iter().any(|&p| p == provider) {
                if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
                    if let Ok(model_info) = registry.get_model(&provider_id, &model_id) {
                        return Some(model_info.id.clone());
                    }
                }
            }
        }
    }

    Some(model_string.clone())
}
```

### 7.2 Medium-Term: Extract Model Selection Service

**New File:** `src/tui/services/modelSelectionService.ts`

```typescript
export interface SelectModelOptions {
  sessionId: string | null;
  selection: ModelSelection;
}

export async function selectModel(options: SelectModelOptions): Promise<void> {
  const { sessionId, selection } = options;
  
  // 1. Configure environment for profile-based models
  if (selection.profileConfig) {
    configureProfileEnvironment(selection.profileConfig);
  }
  
  // 2. Update Rust session if exists
  if (sessionId) {
    if (selection.profileConfig) {
      await sessionSetModelProfile(sessionId, selection.providerId, selection.modelId);
    } else {
      await sessionSetModel(sessionId, selection.providerId, selection.modelId);
    }
  }
  
  // 3. Update store
  useModelStore.getState().setCurrentModel(selection);
  
  // 4. Persist to config
  await persistModelSelection(selection);
}
```

### 7.3 Long-Term: Discriminated Union for Model Types

**File:** `codelet/providers/src/manager.rs`

```rust
#[derive(Debug, Clone)]
pub enum SelectedModel {
    Cloud {
        provider_id: String,
        model_id: String,
        api_model_id: String,  // From registry lookup
    },
    Profile {
        provider_type: ProviderType,
        model_id: String,
    },
    None,
}

impl SelectedModel {
    pub fn api_model_id(&self) -> Option<&str> {
        match self {
            SelectedModel::Cloud { api_model_id, .. } => Some(api_model_id),
            SelectedModel::Profile { model_id, .. } => Some(model_id),
            SelectedModel::None => None,
        }
    }
}
```

---

## 8. Files Requiring Changes

| File | Changes Required |
|------|------------------|
| `codelet/providers/src/manager.rs` | Add provider check before registry lookup; consider `SelectedModel` enum |
| `codelet/providers/src/models/registry.rs` | Split `parse_model_string` into parse + validate |
| `src/tui/components/AgentView.tsx` | Remove `handleSelectModel`, extract env setup |
| `src/tui/services/modelSelectionService.ts` | **NEW FILE** - Centralized model selection |
| `src/tui/services/profileEnvironmentService.ts` | **NEW FILE** - Profile env setup |

---

## 9. Testing Strategy

1. **Unit Tests:**
   - `SelectedModel` enum methods
   - `parse_model_string` pure parsing
   - `validate_provider` validation

2. **Integration Tests:**
   - Switch from cloud to profile model
   - Switch from profile to cloud model
   - Profile model with custom base URL

3. **E2E Tests:**
   - Model selection persistence across sessions
   - Profile environment variables applied correctly

---

## 10. Priority

| Fix | Effort | Impact | Priority |
|-----|--------|--------|----------|
| Quick fix (suppress warning) | Low | Medium | **P1** |
| Extract model selection service | Medium | High | **P2** |
| Delete deprecated handler | Low | Low | **P2** |
| Discriminated union type | High | High | **P3** |
| Split parse/validate | Medium | Medium | **P3** |

---

## Appendix: Log Trace Evidence

From `~/.fspec/fspec.log`:

```
23:25:50.154Z - set_model_direct: set current_provider=OpenAI, selected_model=Qwen/Qwen3-Next-80B-A3B-Instruct-FP8
23:25:52.971Z - parse_model_string: provider 'Qwen' not in registry
23:25:52.971Z - get_openai: model_id=Qwen/Qwen3-Next-80B-A3B-Instruct-FP8, base_url=Some("http://192.168.0.50:8888")
```

The flow is:
1. `set_model_direct` stores the model ID directly ✓
2. Later, `selected_model_id()` tries to parse it as `provider/model` format
3. Registry lookup fails because "Qwen" isn't a cloud provider
4. Falls through to direct return (correct behavior, but logs warning)

The model selection ultimately works, but the warning is noise.
