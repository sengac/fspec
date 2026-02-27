# AST Research: GIT-038 Merge Conflict LLM Context Injection

## Key Code Structures

### mergeWorktreeHandler.ts

**MergeWorktreeContext interface** (line 27):
```typescript
export interface MergeWorktreeContext {
  isIsolated: boolean;
  currentSessionId: string | null;
  repoPath: string;
  setConversation: (updater: (prev: Array<{type: string; content: string}>) => Array<{type: string; content: string}>) => void;
  setInputValue: (value: string) => void;
  cleanupCurrentSessionHandler: () => void;
  onExit: () => void;
  setActionPrompt: (prompt: ActionPrompt | null) => void;
}
```
→ Need to add: `injectLlmContext: (content: string) => void;`

**handleMergeWorktree** (line 60):
- Conflict handling at lines 110-117
- Calls `addStatusMessage(ctx, buildConflictSummary(errorMessage))` on conflict
- No LLM context injection currently

### mergeSummaryFormatting.ts

**parseConflictFiles** (line 72): Extracts file paths from Rust error message
**buildConflictSummary** (line 94): Builds TUI-display conflict summary

### AgentView.tsx

**/merge-worktree handler** (line 3096-3108):
- Calls `handleMergeWorktree({...ctx})` passing all context fields
- Need to add `injectLlmContext` implementation here

### NAPI Bindings

**sessionInjectAssistantMessage** - DOES NOT EXIST YET
- Need new Rust NAPI function in session_manager.rs
- Must push to `inner.messages` (live session context) 
- Must call `handle_output(StreamChunk::text(...))` for UI replay
- Must persist via existing persistence layer

**persistenceAppendMessage** (codelet/napi/index.d.ts line 1231):
- Exists, but only writes to SQLite persistence
- Does NOT push to live Rust session's inner.messages
- LLM won't see message until session is restored

### Rust Session Architecture

**BackgroundSession.inner.messages** - Vec<rig::message::Message>
- Source of truth for LLM context on each API call
- Agent loop locks inner, passes to provider
- Must be updated for live context injection

**session_restore_messages** (session_manager.rs line 6824):
- Heavy function designed for bulk restore
- Parses envelope JSON, builds rig messages, pushes to inner.messages
- Also emits StreamChunks for UI replay
- Too heavy for single message injection
