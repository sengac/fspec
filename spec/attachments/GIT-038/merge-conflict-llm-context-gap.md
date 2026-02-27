# GIT-038: Merge Conflict LLM Context Gap — Analysis & Implementation Plan

## The Problem

When `/merge-worktree` detects file conflicts in an isolated session, the conflict information is displayed to the **user's eyes** via a TUI status message, but **never reaches the LLM's conversation context**. This creates a complete disconnect: the user sees the conflict details and naturally asks the AI to resolve them, but the AI has no idea what they're talking about.

### Evidence (from real usage)

The screenshot below shows the exact scenario. The user is in an isolated session (`Isolation: ACTIVE`, `Worktree: .fspec/worktrees/0d1f2214-6c48-4b5b-973b-ff9a8717321d/`). They edited `README.md` and ran `/merge-worktree`. The TUI correctly displayed:

```
⚠ Merge conflicts detected

  Conflicting files:
      README.md

  These files were modified in both this session and the main worktree.
  Resolve the conflicts, then run /merge-worktree again.
```

The user then asked: **"can you resolve them for me?"**

The LLM's thinking reveals the problem:

> *"The user is asking me to 'resolve them' — but I'm not sure what 'them' refers to. Let me think about the context... There's no obvious antecedent. They might be referring to merge conflicts, but we haven't done any merge operation."*

The LLM sees the system reminder with `Isolation: ACTIVE` and the worktree path, but has **zero knowledge** that:
1. A merge was attempted
2. Conflicts were detected
3. Which files are conflicted
4. What the user wants resolved

---

## User Journey: Current (Broken)

```
1. User works in isolated session, edits files
2. Meanwhile, main worktree also has changes to some of the same files
3. User runs /merge-worktree
4. mergeWorktreeHandler.ts catches a Conflict error from Rust
5. buildConflictSummary() formats a nice display message
6. addStatusMessage() appends { type: 'status', content: '⚠ Merge conflicts...' } to React conversation state
7. User sees the conflict summary in their terminal
8. User says "can you resolve them for me?"
9. This user message goes to the LLM via agent_loop
10. LLM receives: [...system reminders..., user: "can you resolve them for me?"]
11. LLM has NO context about conflicts → confused response
```

**The gap is between steps 6 and 9.** Status messages are React-only UI state — they never enter the Rust session's message history that feeds the LLM.

## User Journey: Target (Fixed)

```
1-6. Same as above
7.  ADDITIONALLY: inject an assistant-role context message into the Rust session
    containing conflict details, file paths, worktree location, and resolution instructions
8.  User sees the conflict summary in their terminal
9.  User says "can you resolve them for me?"
10. LLM receives: [...system reminders..., assistant: "Merge conflicts detected in: README.md. 
    Located at .fspec/worktrees/abc123/. Files contain <<<<<<< markers...", 
    user: "can you resolve them for me?"]
11. LLM understands the context → reads the conflicted files → resolves conflict markers → 
    tells user to run /merge-worktree again
```

---

## Architecture: What Exists Today

### Two Parallel Conversation Channels

The TUI maintains **two separate representations** of the conversation:

#### 1. React Conversation State (UI-only)
- `AgentView.tsx` manages `conversation: ConversationMessage[]` via `setConversation`
- Types: `user-input`, `assistant-text`, `thinking`, `tool-call`, `status`, `watcher-input`
- **`status` messages are display-only** — they render in the TUI but are NOT sent to the LLM
- This is where `addStatusMessage()` writes to (line 46 of `mergeWorktreeHandler.ts`)

#### 2. Rust Session Message History (LLM context)
- Managed by `session_manager.rs` in the NAPI layer
- Messages are persisted via `persist_user_message()` and `persist_assistant_message_internal()`
- The LLM reads from `persistenceGetSessionMessages()` on each turn
- TypeScript can append via `persistenceAppendMessage(sessionId, role, content)`

**The conflict summary currently only enters channel #1 (React state), never channel #2 (Rust persistence).**

### Current Merge Handler Flow (`mergeWorktreeHandler.ts`)

```typescript
// Line 110-117: Conflict error handling
} catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (errorMessage.includes('Conflict')) {
      // GIT-037: Show rich conflict summary with file paths and guidance
      addStatusMessage(ctx, buildConflictSummary(errorMessage));
      // ← NOTHING injected into LLM context here
    } else {
      addStatusMessage(ctx, `Merge failed: ${errorMessage}`);
    }
}
```

### Conflict Error Format (from Rust)

The Rust layer (`codelet_git::merge_session`) throws errors with this format:
```
Conflict detected: ["README.md", "src/utils/helpers.ts"] have been modified in both session and main worktree
```

`parseConflictFiles()` in `mergeSummaryFormatting.ts` already extracts file paths from this format.

### Available NAPI Binding

```typescript
// From codelet/napi/index.d.ts
export declare function persistenceAppendMessage(
  sessionId: string,
  role: string,      // "user" | "assistant" | "system"
  content: string
): NapiAppendResult;
```

This is the most direct way to inject a message into the Rust session's history.

---

## Implementation Plan

### What Needs to Change

#### 1. `MergeWorktreeContext` interface (mergeWorktreeHandler.ts)

Add a new method for injecting context into the LLM:

```typescript
export interface MergeWorktreeContext {
  // ... existing fields ...
  
  /** 
   * GIT-038: Inject a message into the Rust session's message history
   * so the LLM can see it on subsequent turns.
   * This is separate from setConversation (which is UI-only).
   */
  injectLlmContext: (content: string) => void;
}
```

#### 2. Conflict handler in `handleMergeWorktree()` (mergeWorktreeHandler.ts)

After the existing `addStatusMessage()` call, also inject LLM context:

```typescript
if (errorMessage.includes('Conflict')) {
  // GIT-037: Show rich conflict summary in TUI (unchanged)
  addStatusMessage(ctx, buildConflictSummary(errorMessage));
  
  // GIT-038: Inject context into LLM conversation history
  const files = parseConflictFiles(errorMessage);
  const fileList = files 
    ? files.map(f => `  - ${f}`).join('\n')
    : `  (Could not parse file list from error: ${errorMessage})`;
  
  ctx.injectLlmContext(
    `Merge conflicts detected. The following files have conflicts:\n` +
    `${fileList}\n\n` +
    `These files contain git conflict markers (<<<<<<< / ======= / >>>>>>>).\n` +
    `To resolve: read each conflicting file, decide which changes to keep ` +
    `(or combine both), remove the conflict markers, and save the file.\n` +
    `After resolving all conflicts, run /merge-worktree again.`
  );
}
```

#### 3. AgentView.tsx — Wire up the new context method

In the `/merge-worktree` handler block (~line 3097), add the `injectLlmContext` implementation:

```typescript
if (userMessage === '/merge-worktree') {
  await handleMergeWorktree({
    // ... existing fields ...
    injectLlmContext: (content: string) => {
      if (currentSessionId) {
        // Persist as assistant message so LLM sees it in history
        persistenceAppendMessage(currentSessionId, 'assistant', content);
        
        // Also add to React conversation for visual display
        setConversation(prev => [
          ...prev,
          { type: 'assistant-text', content },
        ]);
      }
    },
  });
  return;
}
```

#### 4. `ConversationMessage` type — No changes needed

The existing `assistant-text` type already handles assistant messages. The injected message will render naturally as an assistant response bubble in the conversation view.

### What Does NOT Change

- `buildConflictSummary()` — still produces the TUI status message (unchanged)
- `buildMergeSummary()` — success path (no injection needed)
- `parseConflictFiles()` — already extracts file paths correctly
- `InputTransition.tsx` — action prompt mechanism (unrelated)
- The Rust merge logic — conflict detection unchanged
- System reminder / environment info — worktree path already present

### Message Role: Why "assistant" Not "system"

The injected message should use `role: 'assistant'` because:

1. **Compaction safety**: System messages may be replaced or dropped during context compaction. Assistant messages in the conversation flow are preserved.
2. **Natural conversation flow**: The AI "told" the user about the conflicts — this is semantically an assistant action.
3. **Visibility**: The message appears in the conversation as a normal assistant response, so the user also sees it inline.
4. **Provider compatibility**: All LLM providers handle `user`/`assistant` pairs reliably. Custom system messages mid-conversation have varying support.

### The Worktree Path

The LLM already knows the worktree path from the environment system reminder:

```
Isolation: ACTIVE
Worktree: .fspec/worktrees/0d1f2214-6c48-4b5b-973b-ff9a8717321d/
```

The injected conflict message does NOT need to repeat this — the LLM can correlate. However, for robustness, the conflict file paths should be presented as they appear in the error (relative to the worktree root), which matches how the LLM's file tools (Read/Edit/Write) resolve paths via the session's `effective_cwd`.

---

## Test Scenarios

### Happy Path: Conflict context reaches the LLM
1. Setup isolated session with worktree
2. Create a file conflict between session and main worktree
3. Run `/merge-worktree`
4. Verify: TUI status message contains conflict summary (existing GIT-037 test)
5. Verify: `persistenceAppendMessage` was called with `role='assistant'` and content containing file paths
6. Verify: React conversation state contains an `assistant-text` entry with conflict context

### Negative: No injection on success
1. Run `/merge-worktree` with clean merge
2. Verify: No assistant context message injected (only the action prompt)

### Negative: No injection on non-conflict error
1. Run `/merge-worktree` when worktree is missing
2. Verify: Generic error shown, no assistant context injected

### Integration: LLM can act on the context
1. Merge fails with conflict on `README.md`
2. Assert the persisted assistant message contains "README.md"
3. Assert the persisted assistant message contains "conflict markers"
4. Assert the persisted assistant message contains resolution instructions

---

## Estimated Complexity

**3 story points** — This is a moderate change touching 3 files with clear patterns to follow:
- `mergeWorktreeHandler.ts`: Add ~10 lines for context injection
- `AgentView.tsx`: Add ~8 lines to wire up `injectLlmContext` 
- Tests: Update existing merge worktree test fixtures

The `persistenceAppendMessage` NAPI binding already exists. No Rust changes needed. No new types or components. The main risk is ensuring the injected assistant message doesn't interfere with the agent loop's streaming state.
