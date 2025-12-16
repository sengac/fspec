# REPL Interactive Features Analysis

## Summary

The REPL has partial implementations for interactive features that need completion:
1. **ESC key interruption** - UI shows prompt but action wiring may be incomplete
2. **`/continue` command** - Workflow continuation after interruption

## Current State

### ESC Key Interruption

#### What Exists

**UI Display** (`tui/src/status.rs`):
```rust
// Status line shows "ESC to interrupt" during LLM streaming
```

**Event Detection** (`tui/src/events.rs`):
```rust
// Function to check if key event is ESC
pub fn is_escape_key(event: &KeyEvent) -> bool {
    event.code == KeyCode::Esc
}
```

**Interruption Flag** (`cli/src/interactive.rs`):
```rust
// AtomicBool for signaling interruption
let is_interrupted = Arc::new(AtomicBool::new(false));
```

**Raw Mode** (`cli/src/interactive.rs`):
```rust
// Raw mode enabled/disabled for key detection
crossterm::terminal::enable_raw_mode()?;
// ... streaming ...
crossterm::terminal::disable_raw_mode()?;
```

#### What May Be Missing

1. **Background key listener thread** - Need to spawn a thread/task that:
   - Monitors keyboard input during streaming
   - Sets `is_interrupted.store(true, Ordering::SeqCst)` on ESC

2. **Stream cancellation** - The streaming loop needs to:
   - Check `is_interrupted.load(Ordering::SeqCst)` between chunks
   - Cancel the stream gracefully when interrupted
   - Save partial state for `/continue`

3. **Graceful cleanup** - On interruption:
   - Stop streaming immediately
   - Display interruption message
   - Save conversation state for resumption

### `/continue` Command

#### What May Exist

Need to verify if `/continue` is implemented in command processing:

```rust
// cli/src/interactive.rs - command handling
match input.trim() {
    "/debug" => { /* implemented */ }
    "/claude" => { /* implemented */ }
    "/openai" => { /* implemented */ }
    "/continue" => { /* may be missing or incomplete */ }
    // ...
}
```

#### What's Needed

1. **State persistence on interruption**:
   - Save messages up to interruption point
   - Save partial assistant response (if any)
   - Save tool call state (if mid-execution)

2. **Resumption logic**:
   - Restore conversation from interruption point
   - Re-send to LLM with context about interruption
   - Continue from where streaming stopped

## Implementation Plan

### Phase 1: Audit Current Implementation

```bash
# Search for interruption-related code
rg "interrupt" cli/src/
rg "is_escape" tui/src/
rg "continue" cli/src/
rg "raw_mode" cli/src/
```

### Phase 2: ESC Key Interruption

#### 2.1 Background Key Listener

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

async fn start_key_listener(is_interrupted: Arc<AtomicBool>) {
    tokio::spawn(async move {
        loop {
            if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(key) = crossterm::event::read().unwrap() {
                    if key.code == KeyCode::Esc {
                        is_interrupted.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
    });
}
```

#### 2.2 Stream Cancellation Check

```rust
// In streaming loop
while let Some(chunk) = stream.next().await {
    // Check for interruption
    if is_interrupted.load(Ordering::SeqCst) {
        println!("\n⚠️  Interrupted by user. Use /continue to resume.");
        save_interruption_state(&messages, &partial_response)?;
        break;
    }

    // Process chunk...
    print!("{}", chunk);
}
```

#### 2.3 Interruption State

```rust
#[derive(Debug, Serialize, Deserialize)]
struct InterruptionState {
    messages: Vec<Message>,
    partial_response: Option<String>,
    tool_calls_pending: Vec<ToolCall>,
    timestamp: SystemTime,
}

fn save_interruption_state(state: &InterruptionState) -> Result<()> {
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow!("No home directory"))?
        .join(".codelet")
        .join("interruption_state.json");

    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)?;
    Ok(())
}
```

### Phase 3: `/continue` Command

#### 3.1 Command Handler

```rust
"/continue" => {
    match load_interruption_state() {
        Ok(state) => {
            println!("Resuming from interruption...");

            // Restore messages
            session.messages = state.messages;

            // Add continuation prompt
            let continuation = Message::user(
                "Continue from where you were interrupted. \
                 Your partial response was saved."
            );
            session.messages.push(continuation);

            // Resume streaming
            // ...
        }
        Err(_) => {
            println!("No interruption state found. Nothing to continue.");
        }
    }
}
```

#### 3.2 State Loading

```rust
fn load_interruption_state() -> Result<InterruptionState> {
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow!("No home directory"))?
        .join(".codelet")
        .join("interruption_state.json");

    let json = std::fs::read_to_string(path)?;
    let state: InterruptionState = serde_json::from_str(&json)?;

    // Clear state file after loading (one-time use)
    std::fs::remove_file(path)?;

    Ok(state)
}
```

### Phase 4: Testing

1. **Manual testing scenarios**:
   - Start long streaming response
   - Press ESC during streaming
   - Verify interruption message appears
   - Run `/continue`
   - Verify conversation resumes

2. **Edge cases**:
   - ESC during tool execution
   - ESC when no streaming active
   - `/continue` with no saved state
   - Multiple rapid ESC presses

## Files to Modify

| File | Changes |
|------|---------|
| `cli/src/interactive.rs` | Add key listener, stream cancellation, interruption state |
| `cli/src/session/mod.rs` | Add interruption state persistence |
| `tui/src/events.rs` | May need updates for event handling |
| `cli/src/lib.rs` | Add `/continue` command handler |

## Dependencies

May need to add:
```toml
# cli/Cargo.toml
[dependencies]
crossterm = { version = "0.27", features = ["event-stream"] }
```

## Estimated Effort

- **ESC interruption**: 2-3 hours
- **`/continue` command**: 1-2 hours
- **Testing**: 1 hour
- **Total**: 4-6 hours

## Priority

**Medium** - These are user experience improvements that make the CLI more usable for long-running operations. Not critical for basic functionality, but important for production use.

## Verification Checklist

- [ ] ESC key detected during streaming
- [ ] Streaming stops immediately on ESC
- [ ] Interruption message displayed
- [ ] State saved to `~/.codelet/interruption_state.json`
- [ ] `/continue` loads saved state
- [ ] Conversation resumes correctly
- [ ] State file cleaned up after `/continue`
- [ ] Edge cases handled gracefully
- [ ] No race conditions in key listener
- [ ] Raw mode properly restored on all exit paths
