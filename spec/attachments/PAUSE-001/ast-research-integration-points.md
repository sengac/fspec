# AST Research for PAUSE-001: Interactive Tool Pause

## Summary

This research identifies the key integration points for implementing the tool pause feature.

## 1. Session Status Management (Rust)

### Current SessionStatus enum location
**File: `napi/src/session_manager.rs:38-43`**

```rust
pub enum SessionStatus {
    #[default]
    Idle = 0,
    Running = 1,
    Interrupted = 2,
}
```

### Session status methods
**File: `napi/src/session_manager.rs:850-856`**

```rust
pub fn get_status(&self) -> SessionStatus {
    SessionStatus::from(self.status.load(Ordering::Acquire))
}

pub fn set_status(&self, status: SessionStatus) {
    self.status.store(status as u8, Ordering::Release);
}
```

### NAPI export for status
**File: `napi/src/session_manager.rs:3800`**

```rust
#[napi]
pub fn session_get_status(session_id: String) -> Result<String> {
    let session = get_session(&session_id)?;
    let status = session.get_status();
    Ok(status.as_str().to_string())
}
```

**Integration point**: Add `Paused = 3` to SessionStatus enum, add `pause_state` field to Session struct.

## 2. Tool Progress Pattern (to follow)

The `tool_progress.rs` module provides the pattern to follow for the pause mechanism.

**File: `tools/src/tool_progress.rs:34`**
```rust
static TOOL_PROGRESS_CALLBACK: RwLock<Option<ToolProgressCallback>> = RwLock::new(None);
```

**File: `tools/src/tool_progress.rs:44`**
```rust
pub fn set_tool_progress_callback(callback: Option<ToolProgressCallback>) {
    if let Ok(mut guard) = TOOL_PROGRESS_CALLBACK.write() {
        *guard = callback;
    }
}
```

**File: `tools/src/tool_progress.rs:58`**
```rust
pub fn emit_tool_progress(output_chunk: &str, is_stderr: bool) {
    if let Ok(guard) = TOOL_PROGRESS_CALLBACK.read() {
        if let Some(callback) = guard.as_ref() {
            callback(output_chunk, is_stderr);
        }
    }
}
```

**Pattern to follow**: Create similar `tool_pause.rs` with:
- Global `PAUSE_STATE` with RwLock
- `pause_for_user()` function that blocks using Condvar
- `resume_tool_execution()` function that signals the Condvar

## 3. WebSearchAction Enum

**File: `common/src/web_search.rs:14-41`**

```rust
pub enum WebSearchAction {
    Search { query: Option<String> },
    OpenPage {
        url: Option<String>,
        #[serde(default = "default_headless")]
        headless: bool,
    },
    FindInPage {
        url: Option<String>,
        pattern: Option<String>,
        #[serde(default = "default_headless")]
        headless: bool,
    },
    CaptureScreenshot {
        url: Option<String>,
        output_path: Option<String>,
        full_page: Option<bool>,
        #[serde(default = "default_headless")]
        headless: bool,
    },
    #[serde(other)]
    Other,
}
```

**Integration point**: Add `pause: bool` field to OpenPage, FindInPage, CaptureScreenshot variants.

## 4. React State Hook (TypeScript)

**File: `src/tui/hooks/useRustSessionState.ts:31-38`**

```typescript
export interface RustSessionSnapshot {
  status: string;
  isLoading: boolean;
  model: SessionModel | null;
  tokens: SessionTokens;
  isDebugEnabled: boolean;
  version: number;
}
```

**File: `src/tui/hooks/useRustSessionState.ts:205-219`**

```typescript
function fetchFreshSnapshot(sessionId: string, version: number): RustSessionSnapshot {
  const status = rustStateSource.getStatus(sessionId);
  const isLoading = status === 'running';
  return {
    status,
    isLoading,
    model: rustStateSource.getModel(sessionId),
    tokens: rustStateSource.getTokens(sessionId),
    isDebugEnabled: rustStateSource.getDebugEnabled(sessionId),
    version,
  };
}
```

**Integration point**: Add `isPaused: boolean` and `pauseInfo: PauseInfo | null` to RustSessionSnapshot.

## 5. InputTransition Component (TSX)

**File: `src/tui/components/InputTransition.tsx:64`**

```typescript
export const InputTransition: React.FC<InputTransitionProps> = ({
  isLoading,
  thinkingMessage = 'Thinking',
  thinkingHint = "(Esc to stop | 'Space+Esc' detach)",
  ...
}) => {
```

**Key logic at line 212:**
```typescript
if (animationPhase === 'loading') {
  return <Text dimColor>{currentThinkingText}</Text>;
}
```

**Integration point**: Check `isPaused` in the loading phase and render PauseIndicator instead of ThinkingIndicator.

## 6. Key NAPI Export Patterns

**File: `napi/src/session_manager.rs:3799-3805`**
```rust
#[napi]
pub fn session_get_status(session_id: String) -> Result<String> {
    let session = get_session(&session_id)?;
    let status = session.get_status();
    Ok(status.as_str().to_string())
}
```

**Pattern to follow for new exports**:
- `session_get_pause_state(session_id: String) -> Option<NapiPauseState>`
- `session_pause_resume(session_id: String) -> Result<()>`
- `session_pause_confirm(session_id: String, approved: bool) -> Result<()>`

## 7. File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `codelet/tools/src/tool_pause.rs` | NEW | Core pause mechanism with Condvar |
| `codelet/tools/src/lib.rs` | MODIFY | Export tool_pause module |
| `codelet/napi/src/session_manager.rs` | MODIFY | Add Paused status, pause_state field |
| `codelet/napi/src/tool_pause.rs` | NEW | NAPI bindings for pause functions |
| `codelet/napi/src/lib.rs` | MODIFY | Export tool_pause bindings |
| `codelet/common/src/web_search.rs` | MODIFY | Add pause field to action variants |
| `codelet/tools/src/web_search.rs` | MODIFY | Call pause_for_user when pause=true |
| `src/tui/hooks/useRustSessionState.ts` | MODIFY | Add isPaused, pauseInfo to snapshot |
| `src/tui/components/InputTransition.tsx` | MODIFY | Handle paused state |
| `src/tui/components/PauseIndicator.tsx` | NEW | Pause indicator component |
