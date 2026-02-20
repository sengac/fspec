# GIT-033: File Search Popup Worktree Gap Analysis

## Date: 2026-02-19

## Problem Discovery

When investigating the git worktrees architecture (GIT-018), we discovered that the `@file` search popup in the TUI does not respect isolated session worktree paths.

### Current Implementation

**File:** `src/tui/hooks/useFileSearchInput.ts`

```typescript
const updateFilter = useCallback(async (newFilter: string) => {
  // ...
  const pattern = `**/*${newFilter}*`;
  const result = await callGlobTool(pattern, undefined, true);
  //                                         ^^^^^^^^^
  //                                         Always undefined = current directory
  // ...
}, []);
```

**File:** `src/utils/toolIntegration.ts`

```typescript
export async function callGlobTool(
  pattern: string,
  path?: string,           // ← No session awareness
  caseInsensitive?: boolean
): Promise<GlobResult> {
  return await globSearch(pattern, path, caseInsensitive);
}
```

### The Gap

```
User types "@auth" in isolated session
       │
       ▼
useFileSearchInput.ts
       │
       ▼
callGlobTool(pattern, undefined, true)
       │                ^^^^^^^^
       │                No path = searches cwd
       ▼
globSearch() in codelet-napi
       │
       ▼
Searches: /Users/rquast/projects/fspec/
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
          MAIN PROJECT (wrong!)

But AI tools operate in:
          /Users/rquast/projects/fspec/.fspec/worktrees/<session-id>/
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
          WORKTREE (correct)
```

## Impact

| Scenario | What Happens |
|----------|--------------|
| AI creates new file in worktree | User can't find it via `@search` |
| User references file via `@` | File might not exist in worktree |
| AI deletes file in worktree | User still sees it in `@search` |
| AI modifies file in worktree | User sees old version in preview |

## Solution Architecture

### Required Changes

1. **Pass session ID to `useFileSearchInput`**

```typescript
// src/tui/hooks/useFileSearchInput.ts
export interface UseFileSearchInputOptions {
  inputValue: string;
  onInputChange: (value: string) => void;
  terminalWidth: number;
  disabled?: boolean;
  sessionId?: string;  // NEW: Current session ID for worktree resolution
}
```

2. **Add NAPI function to get effective CWD**

Check if `getSessionEffectiveCwd` is already exposed. From investigation:

```rust
// codelet/napi/src/session_manager.rs - line 5916
fn get_session_effective_cwd(session_id_str: String) -> Option<std::path::PathBuf>
```

This exists as an internal callback but may not be exposed as a NAPI function. Need to add:

```rust
// codelet/napi/src/session_manager.rs
#[napi]
pub fn get_session_effective_cwd_napi(session_id: String) -> Option<String> {
    get_session_effective_cwd(session_id)
        .map(|p| p.to_string_lossy().to_string())
}
```

3. **Update toolIntegration.ts**

```typescript
// src/utils/toolIntegration.ts
import { getSessionEffectiveCwdNapi } from '@sengac/codelet-napi';

export function getEffectiveCwd(sessionId: string): string | null {
  return getSessionEffectiveCwdNapi(sessionId) ?? null;
}
```

4. **Update useFileSearchInput to use effective CWD**

```typescript
// src/tui/hooks/useFileSearchInput.ts
const updateFilter = useCallback(async (newFilter: string) => {
  // Get effective CWD for this session (worktree or project root)
  const searchPath = sessionId 
    ? getEffectiveCwd(sessionId) 
    : undefined;

  const pattern = `**/*${newFilter}*`;
  const result = await callGlobTool(pattern, searchPath, true);
  //                                         ^^^^^^^^^^
  //                                         Now uses worktree path!
}, [sessionId]);
```

5. **Pass sessionId from AgentView**

```typescript
// src/tui/components/AgentView.tsx
const fileSearch = useFileSearchInput({
  inputValue,
  onInputChange: setInputValue,
  terminalWidth: columns,
  disabled: isModelSelectorOpen || isResumeModeActive,
  sessionId: currentSessionId,  // NEW
});
```

## Files to Modify

| File | Change |
|------|--------|
| `codelet/napi/src/session_manager.rs` | Add `get_session_effective_cwd_napi` NAPI export |
| `codelet/napi/index.d.ts` | TypeScript declaration (auto-generated) |
| `src/utils/toolIntegration.ts` | Add `getEffectiveCwd(sessionId)` wrapper |
| `src/tui/hooks/useFileSearchInput.ts` | Accept `sessionId`, use effective CWD for glob |
| `src/tui/components/AgentView.tsx` | Pass `currentSessionId` to `useFileSearchInput` |

## Edge Cases

1. **No session yet** - User hasn't sent first message, session not created
   - Solution: Fall back to project root (current behavior)

2. **Session ID but not isolated** - Non-isolated session
   - Solution: `getEffectiveCwd` returns project root, same as current behavior

3. **Session ended but worktree exists** - PendingMerge state
   - Solution: If viewing completed session, still use its worktree path

## Testing Strategy

1. **Unit test**: `useFileSearchInput` with mocked `getEffectiveCwd`
2. **Integration test**: Create isolated session, write file, verify `@search` finds it
3. **E2E test**: Full flow - create isolated session, AI creates file, user finds via `@`

## Estimate

**3 points** - Clear scope, existing patterns to follow, mostly wiring changes.
