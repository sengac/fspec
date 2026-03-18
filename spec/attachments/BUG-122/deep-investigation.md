# BUG-122: Deep Investigation — Why persistenceGetHistory() Takes 3.8 Seconds

## The Rust Call Chain

```
persistenceGetHistory(project, 100)    [TypeScript — NAPI call]
  → persistence_get_history()          [napi_bindings.rs:273]
    → get_history()                    [mod.rs:514]
      → init_stores()                  [mod.rs:110 — THE BOTTLENECK]
        → MessageStore::new()          [storage.rs:23 — reads ENTIRE messages.jsonl]
        → SessionStore::new()          [storage.rs:199 — reads EVERY session/*.json]
        → BlobStore::new()             [cheap — just sets path]
        → HistoryStore::new()          [history.rs:21 — reads history.jsonl]
      → HISTORY_STORE.lock().get()     [trivial in-memory filter + take(100)]
```

## Why It's Slow: `init_stores()` Initializes EVERYTHING

On the **first call** to ANY persistence function, `init_stores()` initializes **all four**
global singleton stores, even though `get_history` only needs the HistoryStore:

### MessageStore::load_all() — THE #1 SUSPECT (~60-80% of 3.8s)

Reads the **entire** `messages.jsonl` — a single monolithic JSONL file containing
**every message ever stored across all sessions**, including full message content
(often multi-KB assistant responses). Every line is `serde_json::from_str::<StoredMessage>()`
parsing full JSON including content, metadata maps, blob_refs arrays.

With hundreds of sessions × 20+ messages each = thousands of messages, each with
large content strings, deserialized into a `HashMap<Uuid, StoredMessage>` in memory.

### SessionStore::load_all() — SECOND BIGGEST COST (~15-30%)

Does `read_dir("sessions/")`, then for **each .json file**: `read_to_string` + 
`serde_json::from_str::<SessionManifest>()`. With hundreds of sessions, that's
hundreds of individual file open/read/close syscalls.

### HistoryStore::load() — The actual target data (~5-10%)

Reads `history.jsonl` (small entries: display string, timestamp, project, session_id).
Sorts all entries by timestamp descending. This is the only store actually needed.

### BlobStore::new() — Cheap

Just creates a struct with a path. No loading.

## The Absurdity

**A lightweight history query triggers heavyweight initialization of the entire
persistence layer.** The `messages.jsonl` file is an append-only store containing
every message ever written (including multi-KB assistant responses), loaded in its
entirety into a `HashMap` — even though `get_history()` never touches the message
store at all.

## Call Site Analysis

### Production Code: Exactly ONE call site

`AgentView.tsx:1448` — inside `initSession()` useEffect (runs on mount):
```typescript
const history = persistenceGetHistory(currentProjectRef.current, 100);
```

### What the History Data Is Used For

Shell-style command recall triggered by `Shift+↑` and `Shift+↓` in the input field.
That's it. Nothing else.

- ❌ NOT used in any useEffect dependency
- ❌ NOT rendered in any visible UI element on mount
- ❌ NOT used for session state, model selection, or any init logic
- ❌ NOT passed as a prop to any child component
- ✅ Only accessed when user presses Shift+↑ or Shift+↓

### Built-in Graceful Degradation

`handleHistoryPrev` already handles empty history:
```typescript
if (historyEntries.length === 0) {
  return; // Silently does nothing
}
```

## Proposed Fix Architecture

### Immediate Fix: Move Out of initSession Critical Path

Move `persistenceGetHistory()` to a **separate useEffect** that runs independently.
This lets React re-render with the model name (resolved in 28ms) immediately:

```typescript
// Effect 1: Critical path — blocks "Loading..." display
useEffect(() => {
  const initSession = async () => {
    persistenceSetDataDirectory(fspecDir);
    blocklistInit(process.cwd());
    await initializeModels();       // 28ms — sets model name
    setError(null);                 // Triggers re-render → "Loading..." gone
  };
  void initSession();
}, []);

// Effect 2: Non-critical — runs independently, doesn't block UI
useEffect(() => {
  const loadHistory = () => {
    try {
      const history = persistenceGetHistory(project, 100);
      setHistoryEntries(history.map(...));
    } catch (err) { ... }
  };
  // Defer to not block event loop during startup
  const timer = setTimeout(loadHistory, 0);
  return () => clearTimeout(timer);
}, []);
```

### Root Cause Fix: Lazy Store Initialization in Rust

Change `init_stores()` to **only initialize the store that's actually needed**:

```rust
// Instead of init_stores() which inits everything:
fn get_history(...) -> Result<Vec<HistoryEntry>, String> {
    init_history_store()?;  // Only init HistoryStore
    let store = HISTORY_STORE.lock()...;
    Ok(store.get(project, limit)...)
}
```

This would make `persistenceGetHistory()` take ~200ms (history.jsonl only)
instead of 3.8s (all stores).

### Performance Fix: Don't Load Messages Eagerly

`MessageStore::load_all()` reading the entire `messages.jsonl` into memory on
first access is the core performance issue. Options:
1. **Lazy per-session loading** — Only load messages for a specific session on demand
2. **Memory-mapped I/O** — Use mmap instead of reading entire file
3. **Sharded storage** — Per-session message files instead of one monolithic JSONL
4. **Index file** — Maintain an index with offsets, seek to specific messages

## Why Does a NEW Thread Load Messages At All?

This was a key point of confusion. When you start a **brand new** agent thread, the UI
shows "Loading..." for 3.8 seconds. But a new thread has zero messages — so why is it
loading messages?

**Answer: It's NOT intentional.** The message loading is a **side effect**.

The actual chain:
1. `initSession()` calls `persistenceGetHistory()` to populate **shell-style input recall**
   (Shift+↑/↓ to cycle through things you previously typed, like terminal history)
2. `persistenceGetHistory()` calls into Rust, which calls `init_stores()`
3. `init_stores()` is a **monolithic all-or-nothing initializer** — it initializes ALL FOUR
   singleton stores (MessageStore, SessionStore, HistoryStore, BlobStore) on the first call
   to ANY persistence function
4. `MessageStore::new()` eagerly reads the **entire** `messages.jsonl` into memory
5. This file contains every message from every session ever, including multi-KB assistant
   responses — it grows without bound

So: the history feature only needs `HistoryStore`. But calling it triggers `init_stores()`
which drags `MessageStore` along. The 3.8 seconds is almost entirely spent parsing a file
that the history feature never touches.

## Why Load Messages Into Memory At All?

Even when messages ARE needed (e.g., resuming an existing session), the current approach
of loading the **entire** message store into a `HashMap` on first access is fundamentally
flawed. It means startup cost grows linearly with total usage — the more you use the tool,
the slower it gets.

### Current Architecture

`messages.jsonl` is a **single monolithic append-only file** containing every message from
every session ever created. On first access, `MessageStore::load_all()` reads every line,
deserializes each with `serde_json::from_str::<StoredMessage>()`, and inserts into a
`HashMap<Uuid, StoredMessage>` in memory.

This gives O(1) lookups after init, but the init cost is unbounded and proportional to
total historical usage.

### Alternative Approaches

| Approach | How it works | Startup cost | Read cost | Complexity |
|----------|-------------|-------------|-----------|------------|
| **Current (monolithic JSONL)** | Read entire file into HashMap on init | O(N) where N = all messages ever — **grows forever** | O(1) after init | Simple |
| **Per-session files** | `messages/<session-id>.jsonl` | O(1) — only load active session | O(n) where n = session messages | Simple |
| **Index + seek** | Byte-offset index per session, `seek()` to read | O(1) — read index only | O(1) seek + O(n) session messages | Moderate |
| **Memory-mapped I/O** | `mmap()` the file, OS pages in on demand | Near-zero | OS-managed, fast | Complex (append handling) |

### Recommended Fix: Per-Session Message Files

**Per-session sharding** (`messages/<session-id>.jsonl`) is the right fix because:

- **Simplest migration**: Each existing message already has a `session_id` field — just
  split the monolithic file by that field
- **Zero startup cost for new sessions**: No file to read
- **Bounded cost for existing sessions**: Only read messages for the session being resumed,
  not every session ever
- **Natural cleanup**: Deleting a session deletes its message file
- **No index maintenance**: Unlike index+seek, no secondary data structure to keep in sync
- **Append-only still works**: Each session file is still append-only JSONL

### Recommended Fix Architecture

**Do it properly with two Rust changes:**

1. **Lazy per-store initialization**: Replace monolithic `init_stores()` with per-store
   lazy initialization. `get_history()` should only init `HistoryStore`, not all four stores.
   This eliminates the side-effect loading entirely.

2. **Per-session message sharding**: Replace single `messages.jsonl` with per-session files
   in `messages/<session-id>.jsonl`. This makes `MessageStore` init bounded by the active
   session's size, not total historical usage.

Together these changes mean:
- New sessions: near-zero persistence startup (only HistoryStore, which is small)
- Existing sessions: load only that session's messages (tens of messages, not thousands)
- Shell history: loads independently, never triggers message loading
- Total startup: should drop from ~3.8s to <100ms

The TypeScript-side deferral (moving `persistenceGetHistory()` out of `initSession()`) is
still a good idea as defense-in-depth, but it's a **band-aid**, not the fix.
