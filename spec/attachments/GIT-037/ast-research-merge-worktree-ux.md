# AST Research: Merge Worktree UX (GIT-037)

## Files Analyzed

### mergeWorktreeHandler.ts
- **Interface**: `MergeWorktreeContext` (line 19) — Dependencies injected from AgentView
  - `isIsolated: boolean`
  - `currentSessionId: string | null`
  - `repoPath: string`
  - `setConversation` — updater function for conversation array
  - `setInputValue` — sets input value
  - `cleanupCurrentSessionHandler` — cleanup callback
  - `onExit` — exit callback
- **Function**: `handleMergeWorktree(ctx: MergeWorktreeContext)` (line 51) — Main handler
  - Current flow: merge → addStatusMessage (counts only) → cleanup → destroy → exit (immediate)
  - Needs: rich file-by-file summary, deferred exit via action prompt

### InputTransition.tsx
- **Interface**: `InputTransitionProps extends MultiLineInputProps` (line 35)
  - Props: `isLoading`, `thinkingMessage`, `thinkingHint`, `skipAnimation`, `isPaused`, `pauseInfo`, `isCompacting`, `compactionProgress`, `triplePauseSelection`
  - Needs: `actionPrompt`, `clearActionPrompt` props
- **Rendering priority** (early return order):
  1. `isPaused && pauseInfo` → pause rendering (lines 293-358)
  2. `animationPhase === 'loading'` → loading/compaction (lines 361-391)
  3. `animationPhase === 'hiding'` → hide animation (lines 393-396)
  4. `animationPhase === 'showing'` → show animation (lines 399-403)
  5. Complete → full MultiLineInput (lines 406-420)
- Action prompt rendering should slot between 1 and 2

### AgentView.tsx
- Imports `handleMergeWorktree` from handlers (line 160-161)
- Calls handler at line 3094 passing context
- InputTransition rendered at line 6313 with current props
- Needs: `useState<ActionPrompt | null>` and thread through to handler + InputTransition

### Key Patterns
- `useInputCompat` with `InputPriority.MEDIUM` — used for animation interrupt handler (lines 200-236)
- Same pattern needed for action prompt keyboard handler
- `useRef` for guards (e.g. `wasLoadingRef`, `wasCompactingRef`)
- Same pattern for `isClosingRef` double-invoke guard

### sessionService.ts imports
- `inspectSessionChanges` — returns `{filesChanged, filesAdded, filesDeleted}`
- `mergeSessionChanges` — returns `{filesModified, filesAdded, filesDeleted}` (arrays of file paths)
- `destroySession` — async cleanup
