# NAPI Interrupt Fix: Immediate ESC Response During Tool Execution

## Problem Statement

When a user presses ESC during agent execution in the TUI (AgentModal), the interrupt does not take effect immediately. Instead, the user must wait for the current operation (often a tool call) to complete before the agent stops. This creates a frustrating user experience where ESC appears unresponsive.

## Root Cause Analysis

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         RUST (Tokio Runtime)                        │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  run_agent_stream_internal (stream_loop.rs)                  │   │
│  │                                                              │   │
│  │  loop {                                                      │   │
│  │    // Interrupt check ONLY at loop start                     │   │
│  │    if is_interrupted.load(Acquire) { break; }                │   │
│  │                                                              │   │
│  │    // NAPI mode: BLOCKING await on stream.next()             │   │
│  │    chunk = stream.next().await;  // ← BLOCKS FOR ENTIRE      │   │
│  │                                  //   TOOL EXECUTION         │   │
│  │    // Process chunk...                                       │   │
│  │  }                                                           │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    NODE.JS (JavaScript/React)                        │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  AgentModal useInput handler                                 │   │
│  │                                                              │   │
│  │  if (key.escape) {                                           │   │
│  │    sessionRef.current.interrupt();  // Sets AtomicBool       │   │
│  │  }                                                           │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### The Problem

In NAPI mode, the stream loop does a **synchronous await** on `stream.next()`:

```rust
// stream_loop.rs:284-287
_ => {
    // NAPI mode: Direct stream processing (no event handling)
    Some(stream.next().await)  // ← Blocks until next chunk!
}
```

The interrupt flag is only checked at the **top of each loop iteration** (line 246). If a tool call takes 30 seconds, ESC won't work for 30 seconds because:

1. User presses ESC → JavaScript calls `interrupt()` → sets `is_interrupted` to `true`
2. But Rust is blocked in `stream.next().await` waiting for the tool to finish
3. The `is_interrupted` check at line 246 won't run until `stream.next()` returns
4. User experiences ESC as unresponsive

### Contrast with CLI Mode

CLI mode works correctly because it uses `tokio::select!` to race the stream against keyboard events:

```rust
// stream_loop.rs:268-282 - CLI mode
tokio::select! {
    c = stream.next() => Some(c),
    event = es.next() => {
        if key.code == KeyCode::Esc {
            is_interrupted.store(true, Release);  // Immediate!
        }
        None
    }
}
```

## Solution: tokio::sync::Notify

The cleanest fix is to use `tokio::sync::Notify` to allow the JavaScript interrupt signal to immediately wake up the blocked `stream.next().await`.

### Implementation Design

#### 1. Add Notify to CodeletSession

**File: `codelet/napi/src/session.rs`**

```rust
use tokio::sync::Notify;

pub struct CodeletSession {
    inner: Arc<Mutex<codelet_cli::session::Session>>,
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,  // NEW
}

#[napi(constructor)]
pub fn new(provider_name: Option<String>) -> Result<Self> {
    // ... existing code ...
    Ok(Self {
        inner: Arc::new(Mutex::new(session)),
        is_interrupted: Arc::new(AtomicBool::new(false)),
        interrupt_notify: Arc::new(Notify::new()),  // NEW
    })
}

#[napi]
pub fn interrupt(&self) {
    self.is_interrupted.store(true, Release);
    self.interrupt_notify.notify_waiters();  // NEW: Wake the stream loop
}

pub async fn prompt(&self, input: String, callback: StreamCallback) -> Result<()> {
    self.is_interrupted.store(false, Release);

    let is_interrupted = Arc::clone(&self.is_interrupted);
    let interrupt_notify = Arc::clone(&self.interrupt_notify);  // NEW

    // ... existing code ...

    run_agent_stream(
        agent,
        &input,
        &mut session,
        is_interrupted,
        interrupt_notify,  // NEW parameter
        &output
    ).await
}
```

#### 2. Update run_agent_stream Signature

**File: `codelet/cli/src/interactive/stream_loop.rs`**

```rust
pub async fn run_agent_stream<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,  // NEW parameter
    output: &O,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal::<M, O, dyn futures::Stream<Item = TuiEvent> + Unpin + Send>(
        agent,
        prompt,
        session,
        None,
        None,
        is_interrupted,
        Some(interrupt_notify),  // NEW: Pass to internal
        output,
    )
    .await
}
```

#### 3. Use select! in NAPI Mode

**File: `codelet/cli/src/interactive/stream_loop.rs`**

```rust
async fn run_agent_stream_internal<M, O, E>(
    // ... existing params ...
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Option<Arc<Notify>>,  // NEW: Optional for backward compat
    output: &O,
) -> Result<()>
where
    // ... existing bounds ...
{
    // ... existing setup code ...

    loop {
        if is_interrupted.load(Acquire) {
            // ... handle interrupt ...
            break;
        }

        let chunk = match (&mut event_stream, &mut status_interval, &status) {
            (Some(es), Some(si), Some(st)) => {
                // CLI mode: existing tokio::select! with keyboard events
                tokio::select! {
                    c = stream.next() => Some(c),
                    event = es.next() => {
                        if let Some(TuiEvent::Key(key)) = event {
                            if key.code == KeyCode::Esc {
                                is_interrupted.store(true, Release);
                            }
                        }
                        None
                    }
                    _ = si.tick() => {
                        let _ = st.format_status();
                        None
                    }
                }
            }
            _ => {
                // NAPI mode: NEW - race stream against interrupt notification
                match &interrupt_notify {
                    Some(notify) => {
                        let interrupt_fut = notify.notified();
                        tokio::select! {
                            c = stream.next() => Some(c),
                            _ = interrupt_fut => None,  // Wakes IMMEDIATELY
                        }
                    }
                    None => {
                        // Fallback (shouldn't happen in practice)
                        Some(stream.next().await)
                    }
                }
            }
        };

        // ... rest of loop unchanged ...
    }
}
```

## Why This Is The Correct Solution

| Aspect | Benefit |
|--------|---------|
| **Idiomatic** | Standard Tokio pattern for async cancellation |
| **Zero overhead** | No polling, no timers - immediate wake-up via Notify |
| **Minimal changes** | Adds one field, one parameter, one `select!` branch |
| **Correct semantics** | Interrupt takes effect immediately, not after current operation |
| **Backward compatible** | CLI mode unchanged, only NAPI mode gets the fix |
| **Thread-safe** | Notify is designed for cross-thread signaling |

## Sequence Diagram: After Fix

```
Timeline with fix:
═══════════════════════════════════════════════════════════════════════

Rust: tokio::select! {
        stream.next() ──────────────────────────►
        interrupt_notify.notified() ────────────┐
      }                                         │
                                                │
User: ─────────── [ESC pressed] ────────────────┤
                                                │
JS:   interrupt() called ───────────────────────┤
      │                                         │
      ├─ is_interrupted.store(true)             │
      └─ notify.notify_waiters() ───────────────┤
                                                │
Rust: select! wakes up via notified() ──────────┤  ← IMMEDIATE!
      │                                         │
      ▼                                         │
Rust: Loop continues, checks is_interrupted ────┤
      │                                         │
      ▼                                         │
Rust: Breaks out of loop, emits Interrupted ────┤
```

## Files to Modify

1. **`codelet/napi/src/session.rs`**
   - Add `interrupt_notify: Arc<Notify>` field
   - Update constructor to initialize Notify
   - Update `interrupt()` to call `notify_waiters()`
   - Update `prompt()` to pass Notify to stream loop

2. **`codelet/cli/src/interactive/stream_loop.rs`**
   - Add `interrupt_notify: Option<Arc<Notify>>` parameter to `run_agent_stream`
   - Add `interrupt_notify: Option<Arc<Notify>>` parameter to `run_agent_stream_internal`
   - Add `tokio::select!` with `notified()` for NAPI mode

3. **`codelet/cli/src/interactive/mod.rs`** (if needed)
   - Update re-exports if function signature changes

## Testing Strategy

1. **Unit Test**: Verify Notify wakes up select! immediately
2. **Integration Test**: Simulate tool execution with interrupt
3. **Manual Test**: In TUI, start a slow tool call (e.g., file read), press ESC, verify immediate response

## Acceptance Criteria

1. ESC key immediately stops agent execution, even during tool calls
2. No polling or busy-waiting - uses async notification
3. CLI mode behavior unchanged
4. Token tracking and message history correctly updated on interrupt
5. "Agent interrupted" message displayed in TUI
