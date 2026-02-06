# AST Research: Work Unit Context in Session Management

## Overview

This document captures the AST research performed for TUI-059, analyzing the existing session management architecture to understand where work unit context should be integrated.

## Key Files Analyzed

### 1. Rust Session Manager (`codelet/napi/src/session_manager.rs`)

**Key Structures:**
- `SessionManager` (line 3174) - Singleton managing multiple `BackgroundSession` instances
- `SessionStatus` enum (line 46) - States: Idle, Running, Interrupted, Paused, Compacting
- `SessionRole` struct (line 215) - Role metadata for watcher sessions

**Session Creation Flow:**
1. `create_session_with_id()` (line ~3230) creates a new session
2. Calls `inner.inject_context_reminders()` (line 3281) to inject CLAUDE.md and environment info
3. Session is stored in `self.sessions` IndexMap (line 3300)
4. Active session is set via `self.set_active_session(uuid)` (line 3304)

**NAPI Exports:**
- Sessions are tracked by UUID in `sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>`
- Active session tracked by `active_session_id: RwLock<Option<Uuid>>`

### 2. Environment Info (`codelet/cli/src/session/context_gathering.rs`)

**EnvironmentInfo Struct (line 20):**
```rust
pub struct EnvironmentInfo {
    pub platform: String,        // e.g., "linux", "macos", "windows"
    pub arch: String,            // e.g., "x86_64", "aarch64"
    pub shell: Option<String>,   // e.g., "/bin/bash"
    pub user: Option<String>,    // Username
    pub cwd: Option<String>,     // Current working directory
}
```

**to_reminder_content() Method (line 35):**
- Formats info as: `Platform: linux\nArchitecture: x86_64\nShell: /bin/bash\nUser: testuser\nWorking directory: /home/testuser/project`

**TUI-059 Integration Point:**
- Add `work_unit_id: Option<String>` field to `EnvironmentInfo`
- Modify `to_reminder_content()` to include: `Current work unit: AUTH-001`

### 3. Rust State Source (`src/tui/hooks/rustStateSource.ts`)

**RustStateSource Interface (line 42):**
```typescript
export interface RustStateSource {
  getStatus(sessionId: string): string;
  getModel(sessionId: string): SessionModel | null;
  getTokens(sessionId: string): SessionTokens;
  getDebugEnabled(sessionId: string): boolean;
  getPauseState(sessionId: string): PauseInfo | null;
  getBaseThinkingLevel(sessionId: string): number;
  setBaseThinkingLevel(sessionId: string, level: number): void;
  getCompactionProgress(sessionId: string): CompactionProgress | null;
}
```

**TUI-059 Integration Point:**
- Add `getWorkUnitContext(sessionId: string): WorkUnitContext | null`
- Add `setWorkUnitContext(sessionId: string, context: WorkUnitContext | null): void`

### 4. Update Work Unit Status Command (`src/commands/update-work-unit-status.ts`)

**Key Functions:**
- `updateWorkUnitStatus()` (line 91) - Main function that validates and updates status
- System reminders collected in `reminders[]` array (line 695)
- Final system reminder wrapped with `wrapInSystemReminder()` (line 807)

**TUI-059 Integration Point:**
- After status update (line 619), check if there's an active session
- If session exists and work unit differs from session's current context:
  - Generate system reminder about work unit change
  - Update session's work unit context

### 5. NAPI Type Exports (`codelet/napi/src/types.rs`)

NAPI bindings export various types. For TUI-059, need to add:
- `WorkUnitContext` struct (id, title, status)
- NAPI functions: `sessionSetWorkUnitContext`, `sessionGetWorkUnitContext`, `sessionGetActive`

## Implementation Approach

### Layer 1: Rust Changes

1. **New Module:** `codelet/napi/src/work_unit_context.rs`
   - `WorkUnitContext` struct with id, title, status fields
   - Methods: `new()`, `clear()`, `is_set()`, `format_for_environment()`

2. **Session Extension:** Add `work_unit_context: WorkUnitContext` field to `BackgroundSession`

3. **NAPI Exports:** Add functions to `codelet/napi/src/lib.rs`:
   - `session_set_work_unit_context(session_id, id, title, status)`
   - `session_get_work_unit_context(session_id) -> Option<WorkUnitContext>`
   - `session_get_active() -> Option<String>`

4. **Environment Integration:** Modify `codelet/cli/src/session/context_gathering.rs`:
   - Extend `EnvironmentInfo` with `work_unit_id: Option<String>`
   - Update `to_reminder_content()` to include work unit if set

### Layer 2: TypeScript Changes

1. **Types:** `src/tui/types/workUnitContext.ts`
   - `WorkUnitContext` interface
   - `WorkUnitContextChange` interface

2. **Service:** `src/tui/services/workUnitContextService.ts`
   - `setWorkUnitContext(sessionId, context)`
   - `getWorkUnitContext(sessionId)`
   - `getActiveWorkUnitContext()`
   - `detectWorkUnitChange(sessionId, newWorkUnitId, newWorkUnit)`
   - `formatWorkUnitChangeReminder(change)`

3. **Hook:** `src/tui/hooks/useWorkUnitContext.ts`
   - React hook wrapping service layer

4. **Command Integration:** `src/commands/hooks/workUnitStatusHook.ts`
   - `onWorkUnitStatusUpdated(workUnitId, newStatus, workUnitTitle)` - Returns system reminder if work unit changed

## Data Flow

```
BoardView (select work unit)
    ↓
AgentView (session created/resumed)
    ↓
useWorkUnitContext.syncWithSession()
    ↓
workUnitContextService.setWorkUnitContext()
    ↓
NAPI: sessionSetWorkUnitContext()
    ↓
Rust: Session.work_unit_context = WorkUnitContext::new(...)
    ↓
Environment info includes: "Current work unit: AUTH-001"

---

update-work-unit-status command
    ↓
workUnitStatusHook.onWorkUnitStatusUpdated()
    ↓
NAPI: sessionGetActive() → active session ID
    ↓
workUnitContextService.detectWorkUnitChange()
    ↓
If changed: Generate system reminder + update context
```

## Existing Patterns to Follow

1. **Service Layer:** Pure async functions (like `sessionService.ts`)
2. **Hooks:** React integration wrapping services (like `useRustSessionState.ts`)
3. **NAPI Bindings:** Error handling with safe defaults (like `rustStateSource.ts`)
4. **System Reminders:** Use `wrapInSystemReminder()` utility

## Open Questions (Resolved in Example Mapping)

1. ✅ Environment shows only work unit ID, not title or status
2. ✅ Rust SessionManager is source of truth
3. ✅ Notify on work unit change via system reminder
4. ✅ Context set when entering AgentView from BoardView
