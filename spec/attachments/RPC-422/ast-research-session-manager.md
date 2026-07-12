# AST Research: Session Manager Persistence Gap Analysis

## Key Functions Identified

### SessionManager::create_session_with_id
- **Location**: `codelet/sessions/src/session_manager.rs:474`
- **Issue**: Creates BackgroundSession in memory but NEVER calls `codelet_core::persistence::create_session_with_provider()` to write manifest to disk

### SessionManager::destroy_session
- **Location**: `codelet/sessions/src/session_manager.rs:1070`
- **Issue**: Removes from in-memory map but NEVER calls `codelet_core::persistence::delete_session()` to remove manifest from disk

### SessionManager::list_sessions
- **Location**: `codelet/sessions/src/session_manager.rs:358`
- **Issue**: Returns only in-memory sessions, never calls `codelet_core::persistence::list_all_sessions()` to include persisted sessions

### Persistence Functions (Available in codelet_core)
- `create_session_with_provider`: `codelet/core/src/persistence/manifest.rs:555`
- `delete_session`: `codelet/core/src/persistence/manifest.rs:753`
- `list_all_sessions`: `codelet/core/src/persistence/manifest.rs:1116`
- `load_session`: `codelet/core/src/persistence/manifest.rs:579`
- `get_session_message_envelopes`: `codelet/core/src/persistence/manifest.rs:969`

## TypeScript Reference Pattern
`src/tui/services/sessionService.ts:createSession()` does:
1. `persistenceCreateSessionWithProvider(name, project, modelPath)` — creates manifest on disk
2. `sessionManagerCreateWithId(id, modelPath, project, name)` — creates BackgroundSession

The Rust port must mirror this two-step pattern.
