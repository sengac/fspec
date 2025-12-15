# CLI-002: Interactive TUI Technical Decisions

**Story**: CLI-002 - Interactive TUI Agent Mode  
**Date**: 2025-12-02  
**Status**: Hotspots Resolved

---

## Decision Summary

All technical hotspots from Event Storm have been resolved through research and analysis:

| Hotspot | Decision | Rationale |
|---------|----------|-----------|
| Terminal handling | `ratatui` + `crossterm` | Battle-tested in codex, inline mode support |
| ESC key detection | `crossterm::event::EventStream` | Keyboard enhancement flags disambiguate ESC |
| Async coordination | `tokio::select!` | Standard pattern, coordinates streams elegantly |
| Input queue | `tokio::sync::mpsc` channels | Async-native, integrates with event loop |
| Status display | `ratatui` widgets + channels | Rich formatting, non-blocking updates |
| Rig interruption | `Arc<AtomicBool>` + immediate break | Simple, safe, matches codex pattern |
| Provider detection | Extend `ProviderManager` | Add `has_any_provider()` method |

---

## Decision 1: TUI Stack

**Choice**: `ratatui` (v0.29) + `crossterm` (v0.28) + `async-stream` + `tokio`

**Alternatives Considered**:
- ❌ `crossterm` only - Too low-level, reinventing the wheel
- ❌ `rustyline` - Doesn't support inline rich UI, harder to customize
- ❌ `tui-rs` - Deprecated predecessor to ratatui

**Rationale**:
- Codex uses this exact stack in production (41k LOC TUI)
- Supports inline viewport (history preserved in scrollback)
- Rich widget library for status display, formatting
- Panic hook for terminal restoration
- Keyboard enhancement flags for ESC disambiguation
- Future-proof for UI enhancements

**Dependencies**:
```toml
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
async-stream = "0.3"
```

---

## Decision 2: ESC Key Detection

**Choice**: `crossterm::event::EventStream` with keyboard enhancement flags

**Implementation**:
```rust
// Enable keyboard enhancements
execute!(
    stdout(),
    PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
    )
)?;

// Event stream
let mut events = crossterm::event::EventStream::new();
while let Some(Ok(event)) = events.next().await {
    match event {
        Event::Key(key) if key.code == KeyCode::Esc => {
            // Interrupt agent
        }
        // ...
    }
}
```

**Why This Works**:
- Keyboard enhancement flags separate ESC key from ESC sequences
- Works on modern terminals (gracefully degrades on old ones)
- No raw mode hybrid complexity (crossterm handles it)

---

## Decision 3: Async Event Coordination

**Choice**: `tokio::select!` macro coordinating multiple streams

**Pattern** (from codex):
```rust
loop {
    select! {
        // Terminal events (keys, resize, paste)
        Some(event) = event_stream.next() => {
            match event {
                TuiEvent::Key(key) if key.code == KeyCode::Esc => {
                    is_interrupted.store(true, Ordering::Relaxed);
                }
                TuiEvent::Key(key) => handle_input(key),
                TuiEvent::Draw => redraw(),
            }
        }
        
        // Agent streaming (LLM + tool execution)
        Some(chunk) = agent_stream.next() => {
            if !is_interrupted.load(Ordering::Relaxed) {
                process_chunk(chunk);
            } else {
                break; // Stop processing stream
            }
        }
        
        // Status display timer
        _ = status_interval.tick() => {
            update_status_display();
        }
    }
}
```

**Why This Works**:
- Non-blocking: All operations concurrent
- Clean cancellation: Breaking one arm doesn't affect others
- Composable: Easy to add more event sources (provider switch, custom commands)
- Standard Rust async pattern

---

## Decision 4: Input Queue Persistence

**Choice**: `tokio::sync::mpsc::unbounded_channel` for queued inputs

**Implementation**:
```rust
let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel();

// User types while agent is processing
if agent_is_busy {
    input_tx.send(user_input)?;
    println!("⏳ Input queued. Press ESC to interrupt and see queued inputs.");
}

// On interruption, drain queue
if is_interrupted {
    let mut queued = Vec::new();
    while let Ok(input) = input_rx.try_recv() {
        queued.push(input);
    }
    println!("🔄 Queued inputs:\n{}", queued.join("\n"));
}
```

**Why Channels**:
- Async-native (integrates with tokio::select!)
- Thread-safe (could queue from signal handlers if needed)
- Unbounded (user can queue unlimited inputs)
- Survives interruption cycle (channel persists across events)

**Alternative Rejected**: `Vec<String>` + `Mutex`
- More manual synchronization
- Doesn't integrate with async event loop
- No backpressure/notification mechanism

---

## Decision 5: Status Display

**Choice**: `ratatui` widgets with `tokio::time::interval` for updates

**Implementation**:
```rust
use tokio::time::{interval, Duration};

let mut status_interval = interval(Duration::from_secs(1));
let start_time = Instant::now();

loop {
    select! {
        _ = status_interval.tick() => {
            let elapsed = start_time.elapsed().as_secs();
            // Update status line: "🔄 Processing request (5s • ESC to interrupt)"
            print!("\r🔄 Processing request ({}s • ESC to interrupt)", elapsed);
            flush_stdout();
        }
        // ... other event arms
    }
}
```

**Why ratatui Widgets**:
- Can use rich formatting (colors, styles)
- Proper cursor management (no flicker)
- Could add progress bars, spinners later
- Consistent with rest of TUI

**Alternative**: Simple `print!("\r...")` for MVP, upgrade to widgets later

---

## Decision 6: Rig Multi-Turn Interruption

**Choice**: `Arc<AtomicBool>` + immediate break (same as TypeScript codelet)

**Analysis**:
- Rig's `MultiTurnStreamItem` stream yields discrete events
- Breaking between events is safe (no mid-operation state)
- Tools execute atomically (each completes before next stream item)
- Partial results preserved in conversation history

**Implementation**:
```rust
let is_interrupted = Arc::new(AtomicBool::new(false));

// In ESC key handler
is_interrupted.store(true, Ordering::Relaxed);

// In agent stream processing
while let Some(chunk) = agent_stream.next().await {
    if is_interrupted.load(Ordering::Relaxed) {
        break;  // Stop immediately
    }
    process_chunk(chunk);
}
```

**Edge Cases**:
1. **Bash commands**: May continue in background (documented limitation)
   - Phase 2: Process group management to kill children
2. **File writes**: Should use atomic writes (temp + rename)
3. **Long-running tools**: User accepts they might complete

**Why Not "Graceful" Cancellation**:
- Rig doesn't expose tool cancellation API
- Would require major changes to rig or custom tool wrapper
- Over-engineering: Users understand "ESC = stop now"
- Matches codex behavior (they do immediate break)

---

## Decision 7: Startup Card & Provider Detection

**Choice**: Extend `ProviderManager` with `has_any_provider()` method

**Implementation**:
```rust
// In src/providers/manager.rs
impl ProviderManager {
    pub fn has_any_provider(&self) -> bool {
        self.has_claude() || self.has_openai() || 
        self.has_gemini() || self.has_codex()
    }
    
    pub fn list_available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.has_claude() { providers.push("Claude (/claude)".to_string()); }
        if self.has_openai() { providers.push("OpenAI (/openai)".to_string()); }
        if self.has_gemini() { providers.push("Gemini (/gemini)".to_string()); }
        if self.has_codex() { providers.push("Codex (/codex)".to_string()); }
        providers
    }
}

// Startup card display
fn show_startup_card(manager: &ProviderManager) {
    println!("\nCodelet v{}", env!("CARGO_PKG_VERSION"));
    
    if !manager.has_any_provider() {
        println!("Available models: No providers configured");
        println!("Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, or other credentials");
    } else {
        let providers = manager.list_available_providers();
        println!("Available models: {}", providers.join(", "));
    }
    println!();
}
```

---

## Architecture: Event Flow

```
User Input                    Agent Stream
    │                             │
    ├──> TuiEvent::Key ──────────>│
    │         │                    │
    │         ├─ ESC?──> interrupt_flag.set(true)
    │         │                    │
    │         └─ Text ─> input_queue.send()
    │                                │
    │                         is_interrupted?
    │                                │
    │                         ├─ Yes: break
    │                         └─ No: process_chunk()
    │                                │
    └──<──── Display Output ────────┘
```

---

## Implementation Phases

### Phase 1: Minimal Interactive Mode (MVP)
- [ ] Initialize ratatui + crossterm
- [ ] Basic event loop with tokio::select!
- [ ] ESC key interruption
- [ ] Simple input handling (no queue yet)
- [ ] Stream agent responses
- [ ] Panic hook for terminal restoration

**Acceptance Criteria**: Can run agent interactively, interrupt with ESC, type "exit" to quit

### Phase 2: Enhanced UX
- [ ] Input queue with channel
- [ ] Status display with elapsed time
- [ ] Provider switching (/claude, /openai, etc.)
- [ ] Startup card rendering
- [ ] "continue" command to resume

**Acceptance Criteria**: Full feature parity with TypeScript codelet

### Phase 3: Polish (Optional)
- [ ] Rich status widgets (progress bars, spinners)
- [ ] Focus tracking
- [ ] Unix suspend/resume (Ctrl+Z)
- [ ] Better error display with ratatui

---

## Testing Strategy

1. **Unit Tests**: Event handling logic, input queue
2. **Integration Tests**: Terminal state management, interruption
3. **Manual Tests**: 
   - ESC during tool execution
   - Long-running bash commands
   - Provider switching
   - Terminal resize
   - Paste handling

---

## References

- Codex TUI Architecture: `spec/attachments/CLI-002/codex-tui-architecture.md`
- TypeScript Codelet: `/home/rquast/projects/codelet/src/agent/runner.ts`
- Crossterm Docs: https://docs.rs/crossterm/
- Ratatui Docs: https://ratatui.rs/

---

## Conclusion

All technical decisions made. Ready to transform Event Storm → Example Mapping → Scenarios.

The approach is **simple, battle-tested, and future-proof**. We're copying a proven architecture from codex rather than inventing something new.
