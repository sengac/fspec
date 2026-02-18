# AST Research: Tool Pause System Integration Points

## Rust: PauseKind and PauseResponse Enums

**File:** `codelet/tools/src/tool_pause.rs`
- Line 18: `pub enum PauseKind { Continue, Confirm }` - Need to add `Triple` variant
- Line 34: `pub enum PauseResponse { Resumed, Approved, Denied, Interrupted }` - Need to add `AllowOnce`, `AllowSession`

## Rust: Blocklist Middleware Check Functions

**File:** `codelet/tools/src/blocklist/middleware.rs`
- Line 114: `pub fn check_bash_command(command: &str) -> Result<(), BlockedError>` - Need to add prompt handling
- Line 140: `pub fn check_file_path(file_path: &str) -> Result<(), BlockedError>` - Need to add prompt handling
- Line 194: `pub fn allow_for_session(pattern: &str)` - Already exists, will be called on AllowSession
- Line 204: `pub fn is_session_allowed(pattern: &str) -> bool` - Already exists, check before pausing

## Rust: NAPI Session Pause Functions

**File:** `codelet/napi/src/session_manager.rs`
- Line 5665: `pub fn session_pause_resume(session_id: String) -> Result<()>` - Pattern for new function
- Need to add: `pub fn session_pause_triple(session_id: String, choice: String) -> Result<()>`

## TypeScript: PauseKind Type

**File:** `src/tui/types/pause.ts`
- Line 12: `export type PauseKind = 'continue' | 'confirm'` - Need to add `'triple'`

## TypeScript: InputTransition Pause UI

**File:** `src/tui/components/InputTransition.tsx`
- Line 286: `if (isPaused && pauseInfo) { ... }` - Need to add triple case with inline UI

## TypeScript: AgentView Pause Handler

**File:** `src/tui/components/AgentView.tsx`
- Line 5958: `if (displayPauseInfo.kind === 'confirm') { ... }` - Pattern for triple handler
