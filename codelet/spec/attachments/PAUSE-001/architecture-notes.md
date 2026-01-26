# Interactive Tool Pause for Browser Debugging - Architecture Notes

## Problem Statement

When using `WebSearchTool` with `headless: false`, the browser window is visible but the tool completes immediately after extracting content. Users cannot interactively inspect the page, use Chrome DevTools, or debug issues before the tool returns results to the agent.

**Desired behavior**: Tools should be able to pause mid-execution, allowing the user to:
1. See the browser window with the page loaded
2. Open Chrome DevTools and inspect elements/network/console
3. Manually interact with the page if needed
4. Press a key (e.g., Enter) to resume and return results to the agent

**Extended requirement**: This mechanism should be generic and support:
- **Continue pauses** - "I'm waiting, press Enter to continue" (browser debugging)
- **Confirmation pauses** - "Dangerous action, approve (Y) or deny (N)?" (security gates for bash commands, etc.)

## Design Decisions

### Decision 1: Explicit Parameter vs Automatic

**Chosen: Explicit parameter (`pause: true`)**

Two orthogonal concerns:
- `headless: false` = visibility (can I see the browser?)
- `pause: true` = interactivity (should I wait for user before continuing?)

This gives the agent maximum flexibility:
- `{headless: false}` → User can watch the automation run through
- `{headless: false, pause: true}` → User can inspect and interact before continuing

**Bonus**: `pause: true` auto-implies `headless: false` (pausing a headless browser is pointless).

## Current Architecture

### State Flow (TypeScript ↔ Rust)

```
TypeScript (React)
    ↓ useRustSessionState hook (useSyncExternalStore)
    ↓ sessionGetStatus(sessionId)
Rust NAPI (session_manager.rs)
    ↓ SessionStatus enum { Idle=0, Running=1, Interrupted=2 }
    ↓ status.load(Ordering::Acquire)
TypeScript (React)
    ↓ isLoading = status === 'running'
    ↓ InputTransition shows "Thinking..." when isLoading=true
```

### Key Files

| File | Purpose |
|------|---------|
| `codelet/napi/src/session_manager.rs` | `SessionStatus` enum, status getters/setters |
| `codelet/tools/src/tool_progress.rs` | Global callback for streaming tool output |
| `codelet/tools/src/web_search.rs` | `WebSearchTool` implementation |
| `codelet/common/src/web_search.rs` | `WebSearchAction` enum (shared types) |
| `src/tui/hooks/useRustSessionState.ts` | React hook for Rust state sync |
| `src/tui/components/InputTransition.tsx` | "Thinking..." / input animation |
| `src/tui/components/ThinkingIndicator.tsx` | Animated spinner component |

### Existing Patterns to Leverage

1. **`tool_progress.rs`** - Global callback registry pattern (used by `BashTool` for streaming stdout/stderr)
2. **`SessionStatus`** - Atomic enum synced to React via `useSyncExternalStore`
3. **`headless: bool`** - Already supported parameter for visible browser mode

## Proposed Architecture (SOLID/DRY/Composable)

### State Model

```rust
// SessionStatus gets a new variant
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    Paused = 3,  // NEW - check pause_state for details
}

// Two kinds of pauses
pub enum PauseKind {
    /// Simple pause - press Enter to continue
    Continue,
    /// Confirmation required - approve (Y) or deny (N)
    Confirm,
}

// Full pause state
pub struct PauseState {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,  // e.g., the dangerous command text
}

// Response from user
pub enum PauseResponse {
    Resumed,      // User pressed Enter (Continue pause)
    Approved,     // User pressed Y (Confirm pause)
    Denied,       // User pressed N (Confirm pause)
    Interrupted,  // User pressed Esc (either type)
}
```

### Session State (Rust side)

```rust
struct Session {
    status: AtomicU8,                              // SessionStatus (existing)
    pause_state: RwLock<Option<PauseState>>,       // NEW - pause details
    pause_response: (Mutex<Option<PauseResponse>>, Condvar),  // For blocking tool
}
```

### Generic Tool Pause API

**File: `codelet/tools/src/tool_pause.rs`** (NEW)

```rust
//! Generic tool pause mechanism (PAUSE-001)
//!
//! Provides a blocking pause API for any tool that needs user interaction.
//! Supports two pause kinds:
//! - Continue: Simple "press Enter to continue" 
//! - Confirm: Security gate with approve (Y) / deny (N)

/// Request to pause tool execution
pub struct PauseRequest {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

/// Pause tool execution and wait for user response (BLOCKING)
/// 
/// This function:
/// 1. Sets session status to Paused
/// 2. Sets pause_state with request details
/// 3. Blocks on condvar waiting for response
/// 4. Returns the user's response
/// 5. Clears pause_state, sets status back to Running
pub fn pause_for_user(request: PauseRequest) -> PauseResponse {
    // Implementation coordinates with session_manager
}
```

### Usage Examples

**WebSearchTool (Continue pause):**
```rust
if pause {
    match pause_for_user(PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "WebSearch",
        message: format!("Page loaded: {}", url),
        details: None,
    }) {
        PauseResponse::Resumed => { /* continue extracting content */ }
        PauseResponse::Interrupted => return Err(ToolError::Interrupted),
        _ => unreachable!(),  // Continue only returns Resumed or Interrupted
    }
}
```

**BashTool (Confirmation pause for security):**
```rust
if is_blacklisted_command(&command) {
    match pause_for_user(PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Bash",
        message: "Potentially dangerous command",
        details: Some(command.clone()),
    }) {
        PauseResponse::Approved => { /* execute */ }
        PauseResponse::Denied => return Ok(ToolOutput::error("Command denied by user")),
        PauseResponse::Interrupted => return Err(ToolError::Interrupted),
        _ => unreachable!(),  // Confirm only returns Approved, Denied, or Interrupted
    }
}
```

### NAPI Bindings

**File: `codelet/napi/src/tool_pause.rs`** (NEW)

```rust
use napi_derive::napi;

#[napi(string_enum)]
pub enum NapiPauseKind {
    Continue,
    Confirm,
}

#[napi(object)]
pub struct NapiPauseState {
    pub kind: NapiPauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

/// Get current pause state (if paused)
#[napi]
pub fn session_get_pause_state(session_id: String) -> Option<NapiPauseState> { ... }

/// Resume a Continue pause (user pressed Enter)
#[napi]
pub fn session_pause_resume(session_id: String) -> Result<()> { ... }

/// Respond to a Confirm pause (user pressed Y or N)
#[napi]
pub fn session_pause_confirm(session_id: String, approved: bool) -> Result<()> { ... }
```

### React State Sync

**File: `src/tui/hooks/useRustSessionState.ts`** (MODIFY)

```typescript
interface RustSessionSnapshot {
  status: 'idle' | 'running' | 'interrupted' | 'paused';
  isLoading: boolean;      // status === 'running'
  isPaused: boolean;       // status === 'paused'  (NEW)
  pauseInfo: PauseInfo | null;  // (NEW)
  model: SessionModel | null;
  tokens: SessionTokens;
  isDebugEnabled: boolean;
  version: number;
}

interface PauseInfo {
  kind: 'continue' | 'confirm';
  toolName: string;
  message: string;
  details?: string;
}

function fetchFreshSnapshot(sessionId: string, version: number): RustSessionSnapshot {
  const status = rustStateSource.getStatus(sessionId);
  const isLoading = status === 'running';
  const isPaused = status === 'paused';
  const pauseInfo = isPaused ? rustStateSource.getPauseState(sessionId) : null;
  
  return {
    status,
    isLoading,
    isPaused,
    pauseInfo,
    model: rustStateSource.getModel(sessionId),
    tokens: rustStateSource.getTokens(sessionId),
    isDebugEnabled: rustStateSource.getDebugEnabled(sessionId),
    version,
  };
}
```

### UI State Transitions

```
status=running    → "⠋ Thinking... (Esc to stop)"

status=paused, kind=continue → "⏸ WebSearch paused: Page loaded: https://..."
                                "(Press Enter to continue, Esc to stop)"

status=paused, kind=confirm  → "⚠️ Bash: Potentially dangerous command"
                                "rm -rf /important/*"
                                "[Y] Approve  [N] Deny  [Esc] Cancel"

status=idle       → Input placeholder with animation
```

### Keyboard Handling

| Status | Key | Action |
|--------|-----|--------|
| `running` | Esc | Interrupt agent |
| `paused` (continue) | Enter | Resume tool |
| `paused` (continue) | Esc | Interrupt agent |
| `paused` (confirm) | Y | Approve, resume |
| `paused` (confirm) | N | Deny, resume with denial |
| `paused` (confirm) | Esc | Interrupt agent |

### WebSearchAction Enhancement

**File: `codelet/common/src/web_search.rs`** (MODIFY)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchAction {
    Search {
        query: Option<String>,
    },
    OpenPage {
        url: Option<String>,
        #[serde(default = "default_headless")]
        headless: bool,
        /// If true, pause for user inspection before returning
        /// Auto-sets headless=false if pause=true
        #[serde(default)]
        pause: bool,
    },
    FindInPage {
        url: Option<String>,
        pattern: Option<String>,
        #[serde(default = "default_headless")]
        headless: bool,
        #[serde(default)]
        pause: bool,
    },
    CaptureScreenshot {
        url: Option<String>,
        output_path: Option<String>,
        full_page: Option<bool>,
        #[serde(default = "default_headless")]
        headless: bool,
        #[serde(default)]
        pause: bool,
    },
    
    #[serde(other)]
    Other,
}
```

## File Structure (Separation of Concerns)

```
codelet/
├── common/src/
│   └── web_search.rs           # WebSearchAction enum (add pause param)
├── tools/src/
│   ├── lib.rs                  # Re-export tool_pause
│   ├── tool_progress.rs        # Existing: streaming output callback
│   ├── tool_pause.rs           # NEW: generic pause/resume mechanism
│   └── web_search.rs           # WebSearchTool (integrate pause)
├── napi/src/
│   ├── lib.rs                  # Re-export tool_pause bindings
│   ├── session_manager.rs      # Add Paused status, pause_state field
│   └── tool_pause.rs           # NEW: NAPI bindings for pause
└── cli/src/interactive/
    ├── output.rs               # (no changes needed - uses status)
    └── stream_loop.rs          # Handle keyboard for pause responses

src/tui/
├── hooks/
│   └── useRustSessionState.ts  # Add isPaused, pauseInfo to snapshot
├── components/
│   ├── InputTransition.tsx     # Handle paused state (replace Thinking line)
│   ├── PauseIndicator.tsx      # NEW: Renders continue and confirm pauses
│   └── AgentView.tsx           # Wire up pause keyboard handling
└── types/
    └── pause.ts                # NEW: PauseInfo, PauseKind types
```

## Testing Strategy

### Unit Tests

**`tool_pause.rs`:**
- Pause/resume cycle works correctly
- Confirm approve/deny returns correct response
- State is correctly cleared after resume
- Interrupt during pause returns Interrupted

**`session_manager.rs`:**
- Status transitions: Running → Paused → Running
- Pause state is set/cleared correctly
- Concurrent access is safe

### Integration Tests

**`web_search.rs`:**
- OpenPage with `pause: true` sets status to Paused
- OpenPage with `pause: true` auto-forces `headless: false`
- Resume unblocks the tool and returns content
- Interrupt during pause cancels tool execution

**`bash.rs` (future):**
- Blacklisted command triggers Confirm pause
- Approve executes command
- Deny returns error without executing
- Interrupt cancels

### E2E Tests (TUI)

- Status changes to "paused" during pause
- InputTransition shows pause message (replaces Thinking)
- Enter key resumes Continue pause
- Y/N keys respond to Confirm pause
- Esc interrupts during any pause
- Status returns to "running" after resume, then eventually "idle"

## Open Questions

1. ~~Should pause be explicit or automatic?~~ → **Explicit parameter**
2. What key to resume? → **Enter for Continue, Y/N for Confirm**
3. Should we auto-open Chrome DevTools when pausing?
4. Should this apply to all tools? → **Yes, generic mechanism**
5. Multiple pause points per tool execution?
