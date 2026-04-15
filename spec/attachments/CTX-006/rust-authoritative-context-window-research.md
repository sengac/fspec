# CTX-006: Rust-Authoritative Context Window — Research Document

## Current Data Flow (Problematic)

### Path A: TUI Display (Models.dev → TypeScript)

```
models.dev API JSON
  → ModelCache (Rust) → ModelRegistry → ModelInfo.limit.context: u32
    → to_napi_model_info() [napi_bindings.rs:57]
      → NapiModelInfo { context_window: u32 }         ← NAPI boundary
        → TypeScript: ProviderSection.models[].contextWindow
          → useModelSelectorState → ModelSelection.contextWindow
            → AgentView.rustModelInfo.contextWindow
              → SessionHeader badge: [200k] or [1M]
```

### Path B: Compaction Engine (Rust-internal)

```
ProviderManager::context_window()                     [manager.rs:674]
  │
  ├── model_context_window: Option<usize>             (from select_model/set_model_direct)
  │   Priority: NAPI override > models.dev registry > env var > provider constant
  │
  └── → calculate_usable_context(context_window, max_output_tokens)
        → threshold (used for CompactionHook, pre-prompt check, fill %)
```

### Path C: Context Fill Display (Rust → TypeScript)

```
stream_loop.rs:98-116 → emit_context_fill_from_usage()
  fill_percentage = (total_tokens / threshold) * 100
  → StreamEvent::ContextFill { fill_percentage, threshold, context_window }
    → NAPI StreamChunk::ContextFillUpdate
      → AgentView useState: setContextFillPercentage()
        → SessionHeader: [45%] with color coding
```

**The disconnect:** Paths A and B source context_window independently. Path A reads from models.dev at model listing time. Path B reads from ProviderManager (which also reads from models.dev, but via its own resolution chain). Path C displays the percentage from Path B's threshold.

## Proposed Architecture

### Design: Rust emits resolved model metadata after selection

After `session_set_model()` or `session_set_model_profile()` completes, Rust emits a `ModelMetadataUpdate` event containing the **resolved** values:

```rust
// New StreamEvent variant
StreamEvent::ModelMetadata(ModelMetadataInfo {
    model_id: String,
    context_window: u64,        // Resolved by ProviderManager
    max_output_tokens: u64,     // Resolved by ProviderManager
    compaction_threshold: u64,  // New: effective compaction trigger (from CTX-007)
})
```

**Alternatively** (simpler): Expose a NAPI query function:

```rust
#[napi]
pub async fn session_get_model_limits(session_id: String) -> Result<ModelLimits> {
    // Returns resolved context_window, max_output_tokens, compaction_threshold
}
```

The TUI calls this after model selection and stores the result, using it for the `[200k]` badge, debug info, etc.

### Key Principle: Display ≠ Browse

- **Browse context** (model selector list): Can still use models.dev data for display (e.g., "1M context" in the model description). This is informational.
- **Active session context**: Must use Rust-resolved values. The SessionHeader badge, fill percentage, and compaction all derive from the same Rust authority.

## Files to Modify

### Rust Side

| File | Change |
|------|--------|
| `codelet/napi/src/session_manager.rs` | Add `session_get_model_limits()` NAPI function OR emit `ModelMetadata` event |
| `codelet/napi/src/types.rs` | Add `ModelLimits` / `ModelMetadataInfo` NAPI struct |
| `codelet/providers/src/manager.rs` | Add `model_limits()` method returning resolved context_window + max_output + threshold |
| `codelet/cli/src/interactive/stream_loop.rs` | Optionally emit ModelMetadata event at session start |

### TypeScript Side

| File | Change |
|------|--------|
| `src/tui/components/AgentView.tsx` | Replace `currentModel.contextWindow` with Rust-resolved value after model selection |
| `src/tui/components/SessionHeader.tsx` | `contextWindow` prop now comes from Rust, not from providerSections |
| `src/tui/utils/sessionHeaderUtils.ts` | No change (formatContextWindow is purely display) |
| `src/tui/utils/tokenStateUtils.ts` | `calculateContextFillPercentage()` TypeScript fallback should use Rust-resolved values |
| `src/tui/services/modelSelectionService.ts` | After `sessionSetModel()`, query Rust for resolved limits |

## Current TUI Context Window Sourcing (To Replace)

In `AgentView.tsx:1163-1227`, `rustModelInfo` is built by looking up the model in `providerSections`:

```typescript
const rustModelInfo = useMemo(() => {
  // ...
  if (rustModel?.modelId) {
    const model = findModelInProviders(rustModel.providerId, rustModel.modelId);
    if (model) {
      return createModelInfo(model.name, model.reasoning, model.hasVision, model.contextWindow);
      //                                                                     ^^^^^^^^^^^^^^^^
      //                                                                     FROM providerSections (models.dev)
    }
  }
  return createModelInfo(localModelId, currentModel?.reasoning, currentModel?.hasVision, currentModel?.contextWindow);
  //                                                                                     ^^^^^^^^^^^^^^^^
  //                                                                                     FROM ModelSelection (also models.dev)
}, [rustSnapshot.model, currentModel, findModelInProviders]);
```

**After CTX-006:** This should read from Rust-resolved state instead:

```typescript
const rustModelInfo = useMemo(() => {
  if (currentSessionId && rustSnapshot.modelLimits) {
    return createModelInfo(
      displayName,
      reasoning,
      hasVision,
      rustSnapshot.modelLimits.contextWindow  // ← FROM RUST
    );
  }
  // Fallback to providerSections only when no session exists yet
  return createModelInfo(localModelId, ...);
}, [...]);
```

## Impact on Other Components

### Context Fill Percentage (Already Correct)

The fill percentage already comes from Rust via `ContextFillUpdate` events → `updateTokenStateFromChunk()`. No change needed here. The percentage is already calculated using Rust's resolved threshold.

### Model Selector Display

The model selector shows context windows for browsing (e.g., "200k" or "1M" next to each model name). This can keep using models.dev data since it's informational — the user is choosing between models, not looking at the active session's limits.

### Session Resume / Attach

`calculateContextFillPercentage()` in `tokenStateUtils.ts` is a TypeScript fallback for session restore. After CTX-006, this should use the Rust-resolved context_window stored with the session metadata, not re-derive it from the model's models.dev entry.

## Test Strategy

1. **Unit test:** `ProviderManager::model_limits()` returns correct resolved values for each model type
2. **NAPI test:** `session_get_model_limits()` returns the same values as `ProviderManager::model_limits()`
3. **Integration test:** After model selection, SessionHeader shows the Rust-resolved context window, not the models.dev value
4. **Edge case:** Model not in models.dev (custom profile model) — context_window comes from profile config via NAPI
5. **Edge case:** Session resume — context_window restored from session metadata, not re-looked-up from models.dev
