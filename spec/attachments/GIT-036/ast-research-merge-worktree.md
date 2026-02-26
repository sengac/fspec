# GIT-036: Merge Worktree Slash Command — Research & Architecture Analysis

## Overview

This document captures the complete codebase research for implementing the `/merge-worktree` slash command. The command replaces the `/sessions` command and `SessionManagementPanel` with a direct, intent-driven merge-and-close workflow.

---

## 1. Rust Merge Stack (codelet_git)

### 1.1 Core Types

**`SessionResult`** (`codelet/git/src/session_result.rs`):
```rust
pub struct SessionResult {
    pub session_id: String,
    pub diff: String,           // Unified diff of all changes
    pub files_changed: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
    pub base_commit: String,
}
```

**`MergeResult`** (`codelet/git/src/session_status.rs`):
```rust
pub struct MergeResult {
    pub session_id: String,
    pub files_modified: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
}
```

**`GitError::ConflictError`** (`codelet/git/src/error.rs`):
```rust
#[error("Conflict detected: {files:?} have been modified in both session and main worktree")]
ConflictError { files: Vec<String> },
```

### 1.2 Merge Algorithm (`merge_session` in `session_status.rs`, lines 504-525)

The merge follows a 3-step process:

1. **`get_session_diff(repo_path, session_id)`** — Compares base_commit tree against the worktree's working directory. Reads the base commit SHA from `.git/worktrees/<session_id>/HEAD`. Uses `get_tree_files()` for the base commit tree and `collect_worktree_files()` for the current worktree state. Returns `SessionResult` with file categorizations.

2. **`apply_session_changes(repo_path, session_id)`** — The core merge logic:
   - Reads base commit tree, worktree files, AND main repo working directory files
   - Calls `detect_conflicts()` to find files modified in BOTH session and main since base_commit
   - **Conflict detection** (`session_result.rs`, lines 213-250):
     - For files in base_tree: checks if file was changed in BOTH worktree AND main (compared to base content). Deletion in session counts as "changed."
     - For files added in session: checks if same file exists in main with different content
   - If conflicts found → returns `GitError::ConflictError { files }`, worktree stays intact
   - If no conflicts → copies modified/added files to main workdir, removes deleted files, then calls `remove_worktree()` to clean up

3. **`delete_manifest(session_id)`** — Removes `~/.fspec/git-sessions/<session-id>.json`

### 1.3 Key Behavioral Details

- **Worktree is removed on successful merge** — `apply_session_changes()` calls `remove_worktree()` at the end
- **Worktree survives conflicts** — On `ConflictError`, the function returns early before any file operations or worktree removal
- **Clean sessions can be merged** — A merge with no changes succeeds silently and removes the worktree. This is why we need `inspectSessionChanges()` as a gate before calling merge
- **Manifest deleted only on success** — Step 3 only runs after `apply_session_changes()` succeeds
- **Conflict error format reaching TypeScript**: `"Conflict detected: [\"file1.ts\", \"file2.ts\"] have been modified in both session and main worktree"` — The `{files:?}` in the thiserror format uses Rust's Debug trait on `Vec<String>`

### 1.4 Existing Rust Tests (`codelet/git/tests/session_merge_tests.rs`)

5 test scenarios already exist and pass:
- `test_merge_session_changes_to_main` — Modified file applied to main
- `test_merge_session_applies_added_files` — New file appears in main
- `test_merge_session_applies_deleted_files` — File removed from main
- `test_merge_session_fails_on_conflict` — ConflictError with file list
- `test_merge_session_fails_on_added_file_conflict` — New file that already exists with different content
- `test_merge_multiple_sessions_in_order` — Three sessions merged sequentially
- `test_merge_clean_session_removes_worktree` — Empty merge removes worktree

---

## 2. NAPI Layer (`codelet/napi/src/git.rs`)

### 2.1 Exposed Functions

**`inspectSession(repoPath, sessionId) → SessionResultJs`** (line 493):
- Wraps `codelet_git::inspect_session()` which is a read-only alias for `get_session_diff()`
- Returns: `{ sessionId, diff, filesChanged[], filesAdded[], filesDeleted[], baseCommit }`

**`mergeSession(repoPath, sessionId) → MergeResultJs`** (line 517):
- Wraps `codelet_git::merge_session()`
- Returns: `{ sessionId, filesModified[], filesAdded[], filesDeleted[] }`
- Throws: NAPI error with the `GitError` message string on failure

**`applySessionChanges(repoPath, sessionId) → void`** (line 230):
- Lower-level function, used internally by `merge_session`. Not needed directly.

### 2.2 TypeScript Type Definitions (`codelet/napi/index.d.ts`)

```typescript
export interface SessionResultJs {
  sessionId: string;
  diff: string;
  filesChanged: Array<string>;
  filesAdded: Array<string>;
  filesDeleted: Array<string>;
  baseCommit: string;
}

export interface MergeResultJs {
  sessionId: string;
  filesModified: Array<string>;
  filesAdded: Array<string>;
  filesDeleted: Array<string>;
}
```

**Important naming note**: `SessionResultJs` uses `filesChanged` while `MergeResultJs` uses `filesModified` — they mean the same thing (files that existed before and were modified) but the names differ between inspect and merge results.

---

## 3. TypeScript Service Layer (`src/tui/services/sessionService.ts`)

### 3.1 Wrapper Functions (lines 364-439)

```typescript
export function inspectSessionChanges(repoPath: string, sessionId: string): SessionResultJs
  // → wraps inspectSession() NAPI

export function mergeSessionChanges(repoPath: string, sessionId: string): MergeResultJs
  // → wraps mergeSession() NAPI

export function discardSessionChanges(repoPath: string, sessionId: string): DiscardResultJs
  // → wraps discardSession() NAPI
```

### 3.2 `destroySession()` (lines 456-483)

This is the **session lifecycle cleanup** function (TUI-068). It orchestrates:
1. `sessionManagerDestroy(sessionId)` — Destroys the Rust BackgroundSession
2. `fspecState.detachSession(workUnitId)` — Detaches from work unit in fspecStore
3. `sessionState.setCurrentWorkUnit(null, null)` — Clears current work unit in sessionStore
4. `manager.unsubscribeFromSession(sessionId)` — Unsubscribes from GlobalSessionStreamManager

**Critical**: `mergeSession()` (NAPI) removes the git worktree, but `destroySession()` cleans up the Rust session process, store state, and stream subscriptions. Both are needed on successful merge.

---

## 4. AgentView.tsx — Existing Patterns

### 4.1 Key Variables for the Handler

| Variable | Source | Purpose |
|----------|--------|---------|
| `isIsolated` | `useIsIsolated()` hook from `sessionStore` (line 933) | Boolean, whether current session has a worktree |
| `currentSessionId` | Component state | The active session's UUID |
| `currentProjectRef.current` | `useRef(process.cwd())` (line 944) | Repo path for NAPI calls |
| `setConversation` | `useState` (line 856) | Adds messages to the chat display |
| `setInputValue` | Component state | Clears the input box |
| `cleanupCurrentSessionHandler` | `useCallback` (line 1641) | Calls sessionCleanupRef.current() |
| `destroySession` | Imported from `sessionService.ts` (line 200) | Full session teardown |
| `onExit` | Component prop | Returns to board view |

### 4.2 Slash Command Handler Pattern (around line 3095)

The existing `/sessions` handler is the simplest example:
```typescript
if (userMessage === '/sessions') {
  setInputValue('');
  setShowSessionManagementPanel(true);
  return;
}
```

Other slash commands (e.g., `/clear`, `/debug`, `/thinking`) follow the same pattern:
- Match on exact `userMessage` string
- `setInputValue('')` to clear input
- Perform action or show UI
- `return` to prevent message from being sent to LLM

### 4.3 Close Session Flow (`handleExitChoice`, lines 4888-4923)

The "Close Session" path (index === 1):
```typescript
cleanupCurrentSessionHandler();
if (currentSessionId) {
  await destroySession(currentSessionId);
}
onExit();
```

This is exactly what `/merge-worktree` will do after a successful merge.

### 4.4 ConversationMessage Types (`src/tui/types/conversation.ts`)

Messages are typed with a discriminated union:
```typescript
type MessageType = 'user-input' | 'assistant-text' | 'thinking' | 'tool-call' | 'status' | 'watcher-input';

interface ConversationMessage {
  type: MessageType;
  content: string;
  // ... other optional fields
}
```

The handler should use `type: 'status'` for all merge output messages.

---

## 5. Slash Command Registry (`src/tui/utils/slashCommands.ts`)

### 5.1 Current State

```typescript
export const SLASH_COMMANDS: SlashCommand[] = [
  // ... other commands ...
  {
    name: 'sessions',
    description: 'Manage isolated session worktrees',
    requiresSession: false,
  },
];
```

### 5.2 Required Changes

- **Remove**: The `sessions` entry
- **Add**: A `merge-worktree` entry:
  ```typescript
  {
    name: 'merge-worktree',
    description: 'Merge worktree changes and close session',
    // requiresSession defaults to true, which is correct
  }
  ```

---

## 6. SessionManagementPanel — Removal Targets

### 6.1 Files to Delete

| File | Purpose |
|------|---------|
| `src/tui/components/SessionManagementPanel.tsx` | 371 lines — Full panel component |
| `src/tui/components/__tests__/SessionManagementPanel.test.tsx` | Unit tests |
| `src/tui/components/__tests__/SessionManagementPanelKeyboard.test.tsx` | Keyboard interaction tests |

### 6.2 AgentView.tsx References to Remove

| Location | What |
|----------|------|
| Line 157 | `import { SessionManagementPanel } from './SessionManagementPanel'` |
| Line 1099 | `const [showSessionManagementPanel, setShowSessionManagementPanel] = useState(false)` |
| Line 1126 | `showSessionManagementPanel` in modal priority list |
| Lines 3095-3100 | `/sessions` handler block |
| Lines 6018-6027 | `SessionManagementPanel` render block |

### 6.3 What SessionManagementPanel Does (for reference)

The panel (`SessionManagementPanel.tsx`) provides:
- Lists all session worktrees with status (pending_merge, clean, orphaned, active)
- Shows diff details (files modified/added/deleted) for selected session
- Keyboard shortcuts: M=merge, D=discard, P=prune orphaned, R=refresh
- Confirmation dialog before merge/discard/prune
- Uses `listSessionWorktrees`, `inspectSessionChanges`, `mergeSessionChanges`, `discardSessionChanges`, `pruneOrphanedSessions`

The new `/merge-worktree` command replaces only the "merge current session" path. The list/discard/prune functionality is deliberately being dropped per rule [6].

---

## 7. Handler Implementation Plan

```
/merge-worktree
    │
    ├─ setInputValue('')
    │
    ├─ if (!isIsolated)
    │   └─ setConversation type:'status' → "This command is only available in isolated sessions"
    │   └─ return
    │
    ├─ inspectResult = inspectSessionChanges(repoPath, sessionId)
    │
    ├─ if (filesChanged.length + filesAdded.length + filesDeleted.length === 0)
    │   └─ setConversation type:'status' → "Nothing to merge"
    │   └─ return
    │
    ├─ try { mergeResult = mergeSessionChanges(repoPath, sessionId) }
    │   │
    │   ├─ success:
    │   │   ├─ setConversation type:'status' → "✓ Merged: N modified, N added, N deleted"
    │   │   ├─ cleanupCurrentSessionHandler()
    │   │   ├─ await destroySession(currentSessionId)
    │   │   └─ onExit()
    │   │
    │   └─ catch (error):
    │       ├─ if error.message contains 'Conflict':
    │       │   └─ setConversation type:'status' → "Conflict: file1.ts, file2.ts ..."
    │       └─ else:
    │           └─ setConversation type:'status' → "Merge failed: <error message>"
    │       └─ (session stays open)
```

---

## 8. Edge Cases & Design Decisions

1. **Why inspect before merge?** — `mergeSession()` succeeds silently on clean worktrees and removes them. The inspect call is needed to implement rule [9]: "Show 'nothing to merge' and continue session as-is."

2. **Conflict error parsing** — The Rust error format is `"Conflict detected: [\"file1.ts\", \"file2.ts\"] have been modified in both session and main worktree"`. The handler can either display this raw or parse out the file list. Displaying the raw message is simpler and more informative.

3. **No confirmation dialog** — Per rule [8], `/merge-worktree` merges immediately without any confirmation step. The old `SessionManagementPanel` had a Y/N confirm dialog for merges.

4. **handleSubmit is async** — The `handleSubmit` function in AgentView.tsx is already async, so `await destroySession()` works naturally.

5. **Two-phase cleanup on success** — `mergeSession()` removes the git worktree and manifest. `destroySession()` then cleans up the Rust BackgroundSession, detaches from work units, clears store state, and unsubscribes from streams. Both are required.

6. **Non-conflict errors** — The handler should catch any error from `mergeSessionChanges()`, not just conflicts. For example, `WorktreeNotFound` could occur if the worktree was already cleaned up by another process.
