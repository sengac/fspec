# AST Research: NAPI Bindings for Watcher Operations

## Research Goal
Understand existing NAPI patterns and internal methods to implement watcher NAPI bindings.

## Search: WatchGraph methods
```
fspec research --tool=ast --pattern="pub fn add_watcher" --lang=rust --path=codelet/napi/src/
```

Results:
- `session_manager.rs:853` - WatchGraph.add_watcher()
- `session_manager.rs:2368` - SessionManager.add_watcher() (wrapper)

## Search: Existing NAPI patterns
```
fspec research --tool=ast --pattern="#[napi]" --lang=rust --path=codelet/napi/src/session_manager.rs
```

Key existing patterns found:
- `session_manager_create()` - creates session, returns String ID
- `session_manager_create_with_id()` - creates with specific ID
- `session_set_role()` - sets session role (WATCH-004)
- `session_get_role()` - gets session role info

## Key Methods to Use

### WatchGraph (line 838-876)
```rust
pub fn add_watcher(&self, parent_id: Uuid, watcher_id: Uuid) -> Result<(), String>
pub fn get_parent(&self, watcher_id: &Uuid) -> Option<Uuid>
pub fn get_watchers(&self, parent_id: &Uuid) -> Vec<Uuid>
pub fn remove_session(&self, session_id: &Uuid)
```

### SessionManager (line 2210+)
```rust
pub fn add_watcher(&self, parent_id: Uuid, watcher_id: Uuid) -> Result<(), String>
pub fn get_parent(&self, watcher_id: &Uuid) -> Option<Uuid>
pub fn get_watchers(&self, parent_id: &Uuid) -> Vec<Uuid>
```

### BackgroundSession (WATCH-006)
```rust
pub fn receive_watcher_input(&self, input: WatcherInput) -> Result<(), String>
pub fn watcher_broadcast_subscribe(&self) -> broadcast::Receiver<StreamChunk>
```

### WATCH-006 Functions
```rust
pub fn format_watcher_input(input: &WatcherInput) -> String
pub struct WatcherInput { source_session_id, role_name, authority, message }
```

## NAPI Functions to Implement

1. **session_create_watcher(parent_id, model, project, name)**
   - Create session via SessionManager
   - Register in WatchGraph via add_watcher()
   - Subscribe to parent broadcast
   - Return watcher session ID

2. **session_get_parent(session_id)**
   - Call SessionManager.get_parent()
   - Convert Uuid to String or None

3. **session_get_watchers(session_id)**
   - Call SessionManager.get_watchers()
   - Convert Vec<Uuid> to Vec<String>

4. **watcher_inject(watcher_id, message)**
   - Get watcher session and its role
   - Get parent session ID from WatchGraph
   - Create WatcherInput with role info
   - Format via format_watcher_input()
   - Call parent.receive_watcher_input()
