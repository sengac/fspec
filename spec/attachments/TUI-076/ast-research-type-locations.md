# AST Research: Type Locations for TUI-076

## Research Query
Find all definitions of `ModelSelection`, `ProviderSection`, and `ModelSelectorItem` types in `src/tui/`.

## Findings

### Current Type Locations

| Type | File | Line | Export Status |
|------|------|------|---------------|
| `ModelSelection` | `src/tui/components/AgentView.tsx` | 248 | NOT exported (local) |
| `ProviderSection` | `src/tui/components/AgentView.tsx` | 267 | NOT exported (local) |
| `ProviderSection` | `src/tui/types/provider.ts` | 18 | **EXPORTED** |
| `ModelSelectorItem` | `src/tui/components/AgentView.tsx` | 286 | NOT exported (local) |
| `ProviderSectionInfo` | `src/tui/utils/model-selection.ts` | 24 | EXPORTED (different type) |

### Analysis

1. **`ModelSelection`** - Only defined in AgentView.tsx (line 248)
   - Needs to be moved to `types/provider.ts`

2. **`ProviderSection`** - DUPLICATED
   - AgentView.tsx line 267 (local, not exported)
   - types/provider.ts line 18 (exported)
   - AgentView.tsx version should be REMOVED, import from provider.ts

3. **`ModelSelectorItem`** - Only defined in AgentView.tsx (line 286)
   - Needs to be moved to `types/provider.ts`

4. **`ProviderSectionInfo`** - Different type in model-selection.ts
   - NOT the same as `ProviderSection` - different purpose
   - No changes needed

### Import Dependencies

`types/provider.ts` already imports:
- `ProfileConfig` from `../../utils/provider-config`
- `NapiModelInfo` from `@sengac/codelet-napi`

These imports are sufficient for `ModelSelectorItem` which references `NapiModelInfo`.

### Verification Commands

```bash
# Find ModelSelection definitions
grep -rn "interface ModelSelection" src/tui/

# Find ProviderSection definitions  
grep -rn "interface ProviderSection" src/tui/

# Find ModelSelectorItem definitions
grep -rn "type ModelSelectorItem" src/tui/
```

### Target State After TUI-076

| Type | File | Line | Export Status |
|------|------|------|---------------|
| `ModelSelection` | `src/tui/types/provider.ts` | NEW | EXPORTED |
| `ProviderSection` | `src/tui/types/provider.ts` | 18 | EXPORTED (unchanged) |
| `ModelSelectorItem` | `src/tui/types/provider.ts` | NEW | EXPORTED |

AgentView.tsx will import all three from `../types/provider`.
