# AST Research: SessionManager for WatchGraph Integration

## Search: SessionManager struct location
```
fspec research --tool=ast --pattern="pub struct SessionManager" --lang=rust --path=codelet/napi/src/
```

### Result:
```
codelet/napi/src/session_manager.rs:339:1:pub struct SessionManager {
```

## Current SessionManager Structure (lines 339-341)

```rust
/// Singleton session manager
pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, Arc<BackgroundSession>>>,
}
```

## Key Implementation Details

### Singleton Pattern (lines 358-362)
```rust
pub fn instance() -> &'static SessionManager {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<SessionManager> = OnceLock::new();
    INSTANCE.get_or_init(SessionManager::new)
}
```

### Constructor (lines 351-355)
```rust
pub fn new() -> Self {
    Self {
        sessions: RwLock::new(HashMap::new()),
    }
}
```

## Integration Points for WatchGraph

1. **Add `watch_graph: WatchGraph` field** to SessionManager struct (line 340)
2. **Initialize in `new()`** method (line 352-354)
3. **Add delegation methods** for:
   - `add_watcher(parent_id, watcher_id)`
   - `remove_watcher(watcher_id)`
   - `get_watchers(parent_id)`
   - `get_parent(watcher_id)`
   - `cleanup_parent(parent_id)` - called when session is destroyed

## Session Removal Hook

The `destroy_session` method (around line 500+) is where we need to call `watch_graph.cleanup_parent()` to remove all watcher relationships when a parent session is destroyed.

## Thread Safety

SessionManager uses `RwLock<HashMap<...>>` pattern. WatchGraph should follow the same pattern:
- `parent_to_watchers: RwLock<HashMap<Uuid, Vec<Uuid>>>`
- `watcher_to_parent: RwLock<HashMap<Uuid, Uuid>>`
