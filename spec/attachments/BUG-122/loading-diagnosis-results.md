# BUG-122: persistenceGetHistory() Blocks UI for ~5s

## Discovery Method

Used `@microsoft/tui-test` E2E framework to measure actual "Loading..." duration
in a real PTY, then added `performance.now()` timers to `initSession()` and
`initializeModels()`.

## Original Measured Timings (tui-test diagnosis, 2026-03-18)

**"Loading..." visible for 3,785ms on a brand new session via Enter key.**

```
[2026-03-18T02:42:34.635Z] "Loading..." APPEARED after 51ms
[2026-03-18T02:42:38.420Z] "Loading..." DISAPPEARED after 3836ms
VISIBLE DURATION: 3785ms
```

## Original Perf Breakdown (from fspec.log)

```
persistenceSetDataDirectory:      0.1ms
blocklistInit:                    4.8ms
initializeModels:                28.0ms  ← Models ready in 28ms
  loadCloudModels:               12.7ms
  buildCloudSections:             6.5ms
    checkCodexOAuthTokens:        0.6ms
    getProviderConfig(codex):     1.4ms
    checkClaudeOAuthTokens:       0.3ms
    Promise.all (102 providers):  3.4ms
    loadCodexAllowlist:           0.4ms
  loadProfileSections:            7.6ms
  loadPersistedModelString:       0.3ms
persistenceGetHistory:         3775.8ms  ← THE BOTTLENECK
initSession TOTAL:             3809.1ms
```

## Current Scale (2026-04-12 — significantly worse)

| File | Size | Lines |
|------|------|-------|
| `messages/messages.jsonl` | **1.0 GB** | 362,303 |
| `sessions/` | 4,586 files | — |
| `history.jsonl` | 2.6 MB | 8,732 |

The messages file has grown substantially since initial diagnosis. User reports
startup now takes ~5 seconds, consistent with linear growth of MessageStore::load_all().

## Root Cause

In `AgentView.tsx` line ~1413, `initSession()` is a single async function that:

1. `persistenceSetDataDirectory()` — 0.1ms ✓
2. `blocklistInit()` — 4.8ms ✓
3. `initializeModels()` — 28ms ✓ (calls `setCurrentProvider()`)
4. `persistenceGetHistory()` — **~5,000ms** ← blocks event loop
5. `setError(null)` — triggers final re-render

`setCurrentProvider()` is called at step 3, but React can't re-render because
`persistenceGetHistory()` at step 4 is a **synchronous NAPI call** that blocks
the JavaScript event loop. React state updates are batched and only flush when
the event loop is free.

## Why Does persistenceGetHistory() Even Run Here?

`persistenceGetHistory()` loads the last 100 command history entries for the
current project from `HistoryStore` (history.jsonl, 2.6 MB). This populates
Shift+↑/↓ shell-style recall and the /search command.

The HistoryStore itself is fast (~50ms for 8,732 entries). But calling
`get_history()` triggers `init_stores()` which **also** loads:
- MessageStore (1 GB, 362K messages) — **NOT NEEDED** by history
- SessionStore (4,586 files) — **NOT NEEDED** by history
- BlobStore (just a path) — cheap

## Three Store Access Patterns (Corrected Understanding)

### HistoryStore (history.jsonl, 2.6 MB)
- **Used by**: Shift+↑/↓ (handleHistoryPrev/Next), /search (persistenceSearchHistory)
- **Access pattern**: Load ALL entries, filter by current project, in-memory substring search
- **Cross-session**: YES — shows history from ALL sessions for this project
- **Can stay in memory**: YES — 2.6 MB is trivial

### MessageStore (messages.jsonl, 1.0 GB)
- **Used by**: SessionSearch tool (LLM agent), session resume (get_session_messages)
- **Access pattern**: Look up specific messages by UUID via SessionManifest.messages[]
- **Cross-session**: Messages are content-addressed by UUID; shared across sessions
  via Forked/Imported MessageSource. NO session_id field on StoredMessage.
- **Cannot stay in memory**: NO — 1 GB in HashMap consumes ~2 GB with overhead

### SessionStore (sessions/*.json, 4,586 files)
- **Used by**: Session list, session resume, session creation
- **Access pattern**: Individual file read by session ID
- **Can lazy-load**: YES — read individual files on demand

## Proposed Fix (Three Layers)

### Layer 1: Lazy per-store initialization (Rust) — biggest impact on startup
Replace monolithic `init_stores()` with per-store lazy init. `get_history()`
should only init `HistoryStore`, not all four stores. This eliminates the
side-effect that drags in MessageStore.

### Layer 2: Binary index + on-demand loading for MessageStore (Rust)
Keep the monolithic messages.jsonl (content-addressing requires it). Replace
the eager `load_all()` → `HashMap` with:
- A persisted binary index file (`messages.idx`) mapping UUID → byte offset
- On-demand `seek()` to load individual messages
- An LRU cache for recently accessed messages

### Layer 3: TypeScript deferral — defense in depth
Move `persistenceGetHistory()` to its own `useEffect` with `setTimeout(fn, 0)`
so model display is never blocked by history loading.
