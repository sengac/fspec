# TUI-077: Slash Command Removal Plan

## Research Summary

Audited all slash commands in `src/tui/utils/slashCommands.ts` and their handlers in `src/tui/components/AgentView.tsx`.

## Commands to Remove (8 total)

### Category 1: Abandoned Stubs (2)

| Command | Handler | Behavior |
|---------|---------|----------|
| `/mcp` | AgentView.tsx ~line 3394 | Prints "MCP provider management not yet implemented." |
| `/mode` | AgentView.tsx ~line 3407 | Prints "Mode cycling not yet implemented." |

These are pure stubs with no functionality. They exist only in the slash command registry and a trivial handler.

### Category 2: NAPI-006 Message-Level Persistence (3)

| Command | Syntax | Persistence Function |
|---------|--------|---------------------|
| `/fork` | `/fork <index> <name>` | `persistenceForkSession()` |
| `/merge` | `/merge <session> <indices>` | `persistenceMergeMessages()` |
| `/cherry-pick` | `/cherry-pick <session> <index> [--context N]` | `persistenceCherryPick()` |

**Why remove:**
- Speculative features from NAPI-006, never adopted by users
- Cause naming confusion with git worktree operations (e.g., `/merge` sounds like merging worktree changes but actually copies messages between sessions)
- The underlying Rust functions (`persistence_fork_session`, `persistence_merge_messages`, `persistence_cherry_pick`) remain as API surface

**Handler locations in AgentView.tsx:**
- Pre-session path: `/fork` ~line 2085, `/merge` ~line 2134, `/cherry-pick` ~line 2199
- Post-session path: No duplicate handlers (only in pre-session path)

### Category 3: Redundant with Better UX (3)

| Command | Syntax | Superseded By |
|---------|--------|---------------|
| `/switch` | `/switch <name>` | `Shift+←/→` session navigation + `/resume` visual picker |
| `/rename` | `/rename <new-name>` | Auto-rename on first message (line ~2384) |
| `/history` | `/history [--all-projects]` | `Shift+↑/↓` inline history navigation + `/search` interactive search |

**`/switch` analysis:**
- Requires knowing session name exactly (no tab completion)
- `Shift+←/→` navigates sessions without typing anything
- `/resume` shows a visual overlay with all sessions to pick from
- Conclusion: strictly inferior to both alternatives

**`/rename` analysis:**
- Sessions auto-rename to first message text (truncated to 500 chars)
- No practical use case for manual rename discovered
- `persistenceRenameSession` is also called internally for auto-rename (line ~2391), so the import may still be needed

**`/history` analysis:**
- Dumps a plain text list of up to 20 history entries
- `Shift+↑/↓` cycles through history inline (much faster)
- `/search` provides interactive fuzzy search with visual selection
- `/history` is the least useful of the three access patterns

## Commands to Keep (10)

| Command | Reason |
|---------|--------|
| `/model` | Opens model selector dialog |
| `/provider` | Opens provider configuration |
| `/debug` | Toggles debug capture mode |
| `/clear` | Clears conversation history |
| `/compact` | Compacts context window |
| `/thinking` | Sets thinking level (TUI-054) |
| `/anchors` | Views anchor points (TUI-056) |
| `/resume` | Visual session picker overlay |
| `/watcher` | Watcher management overlay (+ `/watcher spawn <slug>`) |
| `/parent` | Switch to parent from watcher session |
| `/blocklist` | Blocklist management (BLOCK-004) |
| `/detach` | Detach session from work unit |
| `/sessions` | Session management panel (GIT-029) |
| `/search` | Interactive history search |

## Files to Modify

### 1. `src/tui/utils/slashCommands.ts`

Remove 8 entries from `SLASH_COMMANDS` array:
- `fork` (line ~59-62)
- `merge` (line ~63-67)
- `cherry-pick` (line ~79-83)
- `switch` (line ~53-57)
- `rename` (line ~68-72)
- `history` (line ~76)
- `mcp` (line ~92)
- `mode` (line ~43)

### 2. `src/tui/components/AgentView.tsx`

**Remove handlers (pre-session handleSubmit path, ~line 1700s):**
- `/history` handler (~line 1804-1836)
- `/switch` handler (~line 2018-2054)
- `/rename` handler (~line 2057-2082)
- `/fork` handler (~line 2085-2131)
- `/merge` handler (~line 2134-2196)
- `/cherry-pick` handler (~line 2199-2260)

**Remove handlers (post-session handleSubmit path, ~line 3100s):**
- `/history` handler (~line 3281-3314)
- `/mcp` handler (~line 3394-3404)
- `/mode` handler (~line 3407-3414)

**Remove imports (if no longer used elsewhere):**
- `persistenceForkSession` — only used by `/fork` handler
- `persistenceMergeMessages` — only used by `/merge` handler
- `persistenceCherryPick` — only used by `/cherry-pick` handler
- `persistenceGetHistory` — used by `/history` handler AND `historyEntries` loading (~line 1636). **KEEP** — still needed for Shift+↑/↓ history.
- `persistenceListSessions` — used by `/merge`, `/cherry-pick`, `/switch`, AND session list loading (~line 4436). **KEEP** — still needed for session management.
- `persistenceRenameSession` — used by `/rename` handler AND auto-rename (~line 2391). **KEEP** — still needed for auto-rename.

**Net import removals:** `persistenceForkSession`, `persistenceMergeMessages`, `persistenceCherryPick`

### 3. `src/tui/__tests__/AgentView-persistence.test.tsx`

Remove test cases:
- "should create a forked session with /fork command" (~line 546)
- "should import messages from another session with /merge command" (~line 584)
- "should import messages with context using /cherry-pick command" (~line 673)

### 4. `src/tui/__tests__/slash-command-palette.test.tsx`

May contain tests referencing removed commands — needs review.

### 5. Feature files (spec/features/)

- `session-persistence-with-fork-and-merge.feature` — may need scenarios removed or feature deprecated
