# AST Research: Session Role and Authority Model

## Search: BackgroundSession struct location
```
fspec research --tool=ast --pattern="pub struct BackgroundSession" --lang=rust --path=codelet/napi/src/
```

### Result:
```
codelet/napi/src/session_manager.rs:106:1:pub struct BackgroundSession {
```

## Analysis

### Location for new types:
- SessionRole struct and RoleAuthority enum should go near line 34-64 where SessionStatus is defined
- This keeps related session types together

### BackgroundSession fields (lines 106-153):
- Add `role: RwLock<Option<SessionRole>>` field after `watcher_broadcast` (around line 152)

### BackgroundSession impl (starts line 155):
- Add `set_role()` and `get_role()` methods near other getter/setter methods

### NAPI bindings location:
- Look at existing NAPI functions like `session_list`, `session_attach` for pattern
- Add `session_set_role` and `session_get_role` functions with #[napi] attribute

## Integration Points

1. **SessionRole struct**: New type with name, description, authority
2. **RoleAuthority enum**: Peer | Supervisor
3. **BackgroundSession.role field**: RwLock<Option<SessionRole>>
4. **BackgroundSession::set_role()**: Method to set role
5. **BackgroundSession::get_role()**: Method to get role  
6. **session_set_role NAPI**: Exposed to TypeScript
7. **session_get_role NAPI**: Exposed to TypeScript
