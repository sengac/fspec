# AST Research: File Search Worktree Integration

## Date: 2026-02-23

## Research Objective

Analyze code structure to validate implementation plan for integrating worktree path resolution into the file search popup.

---

## 1. UseFileSearchInputOptions Interface (Line 15-36)

**File:** `src/tui/hooks/useFileSearchInput.ts`

```typescript
export interface UseFileSearchInputOptions {
  inputValue: string;
  onInputChange: (value: string) => void;
  terminalWidth: number;
  disabled?: boolean;
  // MISSING: sessionId - needs to be added
}
```

**Finding:** Interface needs `sessionId?: string` parameter to support worktree resolution.

---

## 2. useFileSearchInput Function (Line 71-295)

**File:** `src/tui/hooks/useFileSearchInput.ts`

```typescript
export function useFileSearchInput(
  options: UseFileSearchInputOptions
): UseFileSearchInputResult {
  const { inputValue, onInputChange, terminalWidth, disabled = false } = options;
  // sessionId NOT destructured - needs adding
}
```

**Finding:** Function destructures options but doesn't include `sessionId`.

---

## 3. updateFilter Callback (Line 116-143)

**File:** `src/tui/hooks/useFileSearchInput.ts`

```typescript
const updateFilter = useCallback(async (newFilter: string) => {
  // ...
  const pattern = `**/*${newFilter}*`;
  const result = await callGlobTool(pattern, undefined, true);
  //                                         ^^^^^^^^
  //                                         Always undefined - this is the bug
}, []);
```

**Finding:** `callGlobTool` always receives `undefined` for path, meaning it always searches CWD (project root).

**Fix Required:**
```typescript
const updateFilter = useCallback(async (newFilter: string) => {
  // Get effective path for this session
  const searchPath = sessionId 
    ? sessionGetEffectiveCwd(sessionId) ?? undefined 
    : undefined;
  
  const pattern = `**/*${newFilter}*`;
  const result = await callGlobTool(pattern, searchPath, true);
}, [sessionId]);  // Add sessionId to dependency array
```

---

## 4. AgentView Hook Usage (Line 1542-1555)

**File:** `src/tui/components/AgentView.tsx`

```typescript
const fileSearch = useFileSearchInput({
  inputValue,
  onInputChange: setInputValue,
  terminalWidth,
  disabled: isResumeMode || isWatcherMode || isWatcherEditMode || 
            isBlocklistMode || showModelSelector || showSettingsTab || 
            showThinkingLevelDialog,
  // MISSING: sessionId: currentSessionId
});
```

**Finding:** Hook call doesn't pass `sessionId`. `currentSessionId` is available at line 1130.

---

## 5. currentSessionId Availability (Line 1130)

**File:** `src/tui/components/AgentView.tsx`

```typescript
const currentSessionId = useCurrentSessionId();
```

**Finding:** Session ID is readily available via `useCurrentSessionId()` hook. No additional wiring needed.

---

## 6. sessionGetEffectiveCwd NAPI Binding (Line 1612-1614)

**File:** `codelet/napi/index.d.ts`

```typescript
export declare function sessionGetEffectiveCwd(
  sessionId: string
): string | null;
```

**Finding:** NAPI function already exists and is exported. Returns:
- Worktree path for isolated sessions
- Project root for non-isolated sessions
- `null` if session not found

---

## 7. NAPI Export Verification

**File:** `codelet/napi/index.js` (Line 685)

```javascript
export { sessionGetEffectiveCwd }
```

**Finding:** Function is exported and ready to use from TypeScript.

---

## 8. Existing Usage Pattern

**File:** `src/tui/__tests__/isolated-session-file-blocking-e2e.test.ts` (Line 174)

```typescript
const effectiveCwd = sessionGetEffectiveCwd(sessionId);
```

**Finding:** The function is already used in tests with this exact pattern.

---

## Implementation Summary

### Files to Modify

| File | Change |
|------|--------|
| `src/tui/hooks/useFileSearchInput.ts` | Add `sessionId?: string` to interface, import `sessionGetEffectiveCwd`, pass to `callGlobTool` |
| `src/tui/components/AgentView.tsx` | Pass `sessionId: currentSessionId` to `useFileSearchInput` |

### Code Changes

**useFileSearchInput.ts:**

1. Add import:
   ```typescript
   import { sessionGetEffectiveCwd } from '@sengac/codelet-napi';
   ```

2. Add to interface:
   ```typescript
   export interface UseFileSearchInputOptions {
     // ... existing fields
     sessionId?: string;
   }
   ```

3. Destructure in function:
   ```typescript
   const { inputValue, onInputChange, terminalWidth, disabled = false, sessionId } = options;
   ```

4. Update `updateFilter`:
   ```typescript
   const updateFilter = useCallback(async (newFilter: string) => {
     // Get effective path for session (worktree or project root)
     const searchPath = sessionId 
       ? sessionGetEffectiveCwd(sessionId) ?? undefined 
       : undefined;
     
     const pattern = `**/*${newFilter}*`;
     const result = await callGlobTool(pattern, searchPath, true);
     // ... rest unchanged
   }, [sessionId]);
   ```

**AgentView.tsx:**

```typescript
const fileSearch = useFileSearchInput({
  inputValue,
  onInputChange: setInputValue,
  terminalWidth,
  disabled: /* existing conditions */,
  sessionId: currentSessionId,  // ADD THIS LINE
});
```

---

## Edge Cases Validated

| Case | Behavior |
|------|----------|
| `sessionId` is `undefined` | Falls back to CWD (project root) |
| `sessionId` exists but session not found | `sessionGetEffectiveCwd` returns `null`, falls back to CWD |
| Non-isolated session | `sessionGetEffectiveCwd` returns project root |
| Isolated session | `sessionGetEffectiveCwd` returns worktree path |

---

## Test Strategy

1. **Unit Test:** Mock `sessionGetEffectiveCwd` and verify correct path passed to `callGlobTool`
2. **Integration:** Use real NAPI with isolated session to verify worktree search

---

## Complexity Assessment

**Estimate: 3 points** - Straightforward wiring change with clear existing patterns.
