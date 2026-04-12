# BUG-122: Deep Investigation — Architecture & Fix Design

## The Rust Call Chain

```
persistenceGetHistory(project, 100)    [TypeScript — NAPI call]
  → persistence_get_history()          [napi_bindings.rs]
    → get_history()                    [mod.rs:517]
      → init_stores()                  [mod.rs:113 — THE BOTTLENECK]
        → MessageStore::new()          [storage.rs:24 — reads ENTIRE 1GB messages.jsonl]
        → SessionStore::new()          [storage.rs — reads 4,586 session/*.json files]
        → BlobStore::new()             [cheap — just sets path]
        → HistoryStore::new()          [history.rs:21 — reads 2.6MB history.jsonl]
      → HISTORY_STORE.lock().get()     [trivial in-memory filter + take(100)]
```

## Why Per-Session Message Sharding Was REJECTED

Initial analysis proposed splitting messages.jsonl into per-session files.
This was rejected after deeper investigation revealed:

1. **StoredMessage has NO session_id field** (types.rs:62-80). It's content-
   addressed by UUID only.

2. **Messages are shared across sessions** via `MessageSource::Forked` and
   `MessageSource::Imported` (types.rs:84-93). A single message UUID can
   appear in multiple SessionManifest.messages[] arrays.

3. **Shift+↑/↓ and /search use HistoryStore, NOT MessageStore**. The command
   history feature already loads from `history.jsonl` which contains entries
   from ALL sessions — this is by design and must stay cross-session.

4. **SessionSearch (LLM tool)** accesses MessageStore through per-session
   lookups: it iterates a session's MessageRef[] UUIDs and calls `get(uuid)`.
   It also does cross-session search by iterating all sessions, loading each
   session's messages, and applying ripgrep matching.

Per-session sharding would break content-addressing and require duplicating
shared messages across files.

## Correct Fix: Three Layers

### Layer 1: Lazy Per-Store Initialization (mod.rs)

Replace the monolithic `init_stores()` (called from 30 functions) with
individual per-store initializers:

```rust
fn init_message_store() -> Result<(), String> {
    let mut store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() { *store = Some(MessageStore::new()?); }
    Ok(())
}

fn init_session_store() -> Result<(), String> {
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() { *store = Some(SessionStore::new()?); }
    Ok(())
}

fn init_blob_store() -> Result<(), String> {
    let mut store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() { *store = Some(BlobStore::new()?); }
    Ok(())
}

fn init_history_store() -> Result<(), String> {
    let mut store = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() { *store = Some(HistoryStore::new()?); }
    Ok(())
}
```

Each of the 30 call sites updated to init only the stores they need:

| Function group | Needs |
|----------------|-------|
| `get_history`, `add_history`, `search_history` | HistoryStore only |
| `store_message`, `get_message`, `get_session_messages` | MessageStore + BlobStore |
| `create_session`, `load_session`, `list_sessions` | SessionStore only |
| `append_message`, `append_message_with_metadata` | MessageStore + SessionStore + BlobStore |

### Layer 2: Binary Index + On-Demand Loading for MessageStore (storage.rs)

Keep the monolithic messages.jsonl. Replace `load_all()` → `HashMap` with:

**Binary index file format** (`messages.idx`):
```
[magic: 4 bytes "MIDX"]
[version: 4 bytes (u32 LE)]
[data_file_size: 8 bytes (u64 LE)]   ← for staleness detection
[entry_count: 4 bytes (u32 LE)]
[entries: entry_count × 28 bytes each]
  [uuid: 16 bytes]
  [byte_offset: 8 bytes (u64 LE)]
  [byte_length: 4 bytes (u32 LE)]
```

For 362K messages: 20 bytes header + 362,303 × 28 bytes = ~10.1 MB

**New MessageStore struct**:
```rust
pub struct MessageStore {
    messages_dir: PathBuf,
    messages_file: PathBuf,
    index_file: PathBuf,
    /// UUID → (byte_offset, byte_length)
    index: HashMap<Uuid, IndexEntry>,
    /// LRU cache for recently accessed messages
    hot_cache: parking_lot::Mutex<lru::LruCache<Uuid, StoredMessage>>,
    /// Open file handle for seeking
    reader: parking_lot::Mutex<Option<File>>,
}

struct IndexEntry {
    byte_offset: u64,
    byte_length: u32,
}
```

**Startup sequence**:
1. Check if `messages.idx` exists and is valid (magic + version)
2. Compare stored `data_file_size` with actual `messages.jsonl` size
3. If equal: load binary index (~15ms for 10 MB) — done
4. If actual > stored: incrementally scan only the new bytes (appended since
   last index build). Extract UUIDs via lightweight pattern matching on each
   new line, record byte offsets. Append new entries to index.
5. If actual < stored or missing: full rebuild by scanning entire file.
   This only happens on first upgrade or after cleanup_orphans rewrites.
6. Save updated `messages.idx`

**get() method**:
```rust
pub fn get(&self, id: Uuid) -> Option<StoredMessage> {
    // Check LRU cache first
    if let Some(msg) = self.hot_cache.lock().get(&id) {
        return Some(msg.clone());
    }
    // Look up in index
    let entry = self.index.get(&id)?;
    // Seek and read from file
    let msg = self.read_at(entry.byte_offset, entry.byte_length)?;
    // Insert into LRU cache
    self.hot_cache.lock().put(id, msg.clone());
    Some(msg)
}
```

**store() method** (append):
```rust
pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String> {
    // Serialize and append to messages.jsonl (same as current)
    let offset = file_length;
    writeln!(file, "{}", json)?;
    let length = json.len() + 1; // +1 for newline
    // Update in-memory index
    self.index.insert(id, IndexEntry { byte_offset: offset, byte_length: length as u32 });
    // Update LRU cache
    self.hot_cache.lock().put(id, msg.clone());
    // Periodically flush index to disk (every N writes or on drop)
    Ok(id)
}
```

### Layer 3: TypeScript Deferral (AgentView.tsx)

Move `persistenceGetHistory()` out of `initSession()`:

```typescript
// Effect 1: Critical path — ~28ms
useEffect(() => {
    const initSession = async () => {
        persistenceSetDataDirectory(fspecDir);
        blocklistInit(process.cwd());
        await initializeModels();   // 28ms — sets model name
        setError(null);             // "Loading..." disappears
    };
    void initSession();
}, []);

// Effect 2: Non-critical — deferred, doesn't block UI
useEffect(() => {
    const timer = setTimeout(() => {
        try {
            const history = persistenceGetHistory(project, 100);
            setHistoryEntries(history.map(...));
        } catch (err) { /* log */ }
    }, 0);
    return () => clearTimeout(timer);
}, []);
```

## Expected Results

| Scenario | Before | After |
|----------|--------|-------|
| New session startup | ~5s | <100ms (Layer 1+3) |
| Session resume | ~5s | ~50ms (Layer 2: index lookup + seek for session's messages) |
| Shell history (Shift+↑) | Available after 5s (blocked by MessageStore) | Available after ~50ms (only HistoryStore) |
| /search command | Available after 5s | Available after ~50ms (uses HistoryStore) |
| SessionSearch tool | Needs all 362K in HashMap | On-demand seek + LRU cache |
| MessageStore init | Parse 1 GB into HashMap | Load 10 MB index (~15ms) |
| RAM at steady state | ~2 GB (full HashMap) | ~10 MB index + LRU cache |

## Data Preservation

All 362,303 message lines are preserved in the monolithic messages.jsonl.
The binary index is a secondary derived file that can be rebuilt from the
JSONL file at any time. No migration, no data movement.

## Cross-Session Access Preserved

- **Shift+↑/↓**: Uses HistoryStore (unchanged) — always cross-session
- **/search**: Uses HistoryStore (unchanged) — always cross-session
- **SessionSearch**: Iterates sessions from SessionStore, loads each session's
  messages by UUID from MessageStore via index+seek — cross-session by design
- **Message sharing**: Forked/Imported messages referenced by UUID from
  multiple sessions — works with content-addressed store + index
