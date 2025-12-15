# Codex TUI Architecture Analysis

**Repository**: https://github.com/openai/codex  
**Analyzed**: codex-rs/tui (41,134 LOC)  
**Date**: 2025-12-02

## Executive Summary

Codex uses a sophisticated **inline-first TUI** with optional alternate screen mode, built on `ratatui` + `crossterm` + async event streams. The architecture prioritizes history preservation (inline scrollback) while providing rich terminal UI capabilities.

---

## Core Technology Stack

### Dependencies
```toml
ratatui = "0.29.0"           # TUI framework (widgets, rendering, buffers)
crossterm = "0.28.1"         # Terminal control (raw mode, events, cursor)
tokio = { ... }              # Async runtime
tokio-stream = { ... }       # Async stream utilities
async-stream = { ... }       # stream! macro for event handling
```

### Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  Application Layer (App, AppEvent enum)        │
├─────────────────────────────────────────────────┤
│  TUI Layer (Tui struct, event stream)          │
├─────────────────────────────────────────────────┤
│  Custom Terminal (inline viewport management)   │
├─────────────────────────────────────────────────┤
│  Ratatui Backend (CrosstermBackend)            │
├─────────────────────────────────────────────────┤
│  Crossterm (raw mode, events, ANSI codes)      │
└─────────────────────────────────────────────────┘
```

---

## Key Architectural Patterns

### 1. Inline Viewport (Default Mode)

**Critical**: Codex defaults to **inline viewport**, NOT alternate screen.

```rust
/// Initialize the terminal (inline viewport; history stays in normal scrollback)
pub fn init() -> Result<Terminal> {
    set_modes()?;  // Enable raw mode, keyboard enhancements
    set_panic_hook();  // Terminal restoration on crash
    
    let backend = CrosstermBackend::new(stdout());
    let tui = CustomTerminal::with_options(backend)?;  // NO EnterAlternateScreen!
    Ok(tui)
}
```

**Why inline?**
- History preserved in terminal scrollback
- Feels like native CLI (not full-screen app)
- Can copy/paste previous output
- Matches UX of TypeScript codelet

**Alternate screen is OPTIONAL**:
```rust
pub fn enter_alt_screen(&mut self) -> Result<()> {
    execute!(self.terminal.backend_mut(), EnterAlternateScreen)?;
    // ... save viewport, switch to full screen
}

pub fn leave_alt_screen(&mut self) -> Result<()> {
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
    // ... restore inline viewport
}
```

### 2. Async Event Stream Architecture

**Core pattern**: `tokio::select!` coordinates multiple async event sources.

```rust
pub fn event_stream(&self) -> Pin<Box<dyn Stream<Item = TuiEvent> + Send>> {
    let mut crossterm_events = crossterm::event::EventStream::new();
    let mut draw_rx = self.draw_tx.subscribe();
    
    let event_stream = async_stream::stream! {
        loop {
            select! {
                // Terminal input events (keys, paste, resize, focus)
                Some(Ok(event)) = crossterm_events.next() => {
                    match event {
                        Event::Key(key_event) => yield TuiEvent::Key(key_event),
                        Event::Paste(pasted) => yield TuiEvent::Paste(pasted),
                        Event::Resize(_, _) => yield TuiEvent::Draw,
                        Event::FocusGained => yield TuiEvent::Draw,
                        Event::FocusLost => { /* track focus state */ }
                        _ => {}
                    }
                }
                // Draw requests (triggered by app state changes)
                result = draw_rx.recv() => {
                    yield TuiEvent::Draw;
                }
            }
        }
    };
    Box::pin(event_stream)
}
```

**Event types**:
```rust
pub enum TuiEvent {
    Key(KeyEvent),    // Keyboard input
    Paste(String),    // Terminal paste event
    Draw,             // Redraw request
}
```

**This pattern enables**:
- Non-blocking keyboard input
- Agent streaming concurrent with user input
- Interruption via ESC key while agent is streaming
- Periodic status updates without blocking

### 3. Terminal State Management

**Modes**:
```rust
pub fn set_modes() -> Result<()> {
    execute!(stdout(), EnableBracketedPaste)?;
    enable_raw_mode()?;
    
    // Enable keyboard enhancements (disambiguate ESC codes)
    let _ = execute!(
        stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    );
    
    let _ = execute!(stdout(), EnableFocusChange);
    Ok(())
}
```

**Restoration** (critical for crash recovery):
```rust
pub fn restore() -> Result<()> {
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    execute!(stdout(), DisableBracketedPaste)?;
    let _ = execute!(stdout(), DisableFocusChange);
    disable_raw_mode()?;
    let _ = execute!(stdout(), crossterm::cursor::Show);
    Ok(())
}

fn set_panic_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore(); // Restore terminal even on panic!
        hook(panic_info);
    }));
}
```

### 4. Graceful Keyboard Enhancement Degradation

**Problem**: Older terminals don't support keyboard enhancements.

**Solution**: Try to enable, silently ignore failures.

```rust
// May fail on legacy terminals (Windows console) - that's OK
let _ = execute!(stdout(), PushKeyboardEnhancementFlags(...));

let enhanced_keys_supported = supports_keyboard_enhancement().unwrap_or(false);
```

### 5. Unix Suspend/Resume (Ctrl+Z)

**Codex handles job control gracefully**:

```rust
#[cfg(unix)]
if SUSPEND_KEY.is_press(key_event) {  // Ctrl+Z
    let _ = suspend_context.suspend(&alt_screen_active);
    // Process suspended, terminal restored
    // ... user types 'fg' to resume
    // Terminal re-initialized, continue execution
    yield TuiEvent::Draw;
}
```

This requires careful orchestration:
1. Disable raw mode before suspend
2. Send SIGTSTP to self
3. On resume (SIGCONT), re-enable raw mode
4. Redraw UI

### 6. Focus Tracking

**Tracks whether terminal window is focused**:

```rust
Event::FocusGained => {
    terminal_focused.store(true, Ordering::Relaxed);
    crate::terminal_palette::requery_default_colors();  // Refresh colors
    yield TuiEvent::Draw;
}
Event::FocusLost => {
    terminal_focused.store(false, Ordering::Relaxed);
}
```

**Use case**: Pause animations, reduce CPU when unfocused.

### 7. Frame Scheduling

**Efficient redraw coordination**:

```rust
pub struct FrameRequester {
    frame_schedule_tx: tokio::sync::mpsc::UnboundedSender<Instant>,
}

impl FrameRequester {
    pub fn schedule_frame(&self) {
        let _ = self.frame_schedule_tx.send(Instant::now());
    }
    
    pub fn schedule_frame_in(&self, dur: Duration) {
        let _ = self.frame_schedule_tx.send(Instant::now() + dur);
    }
}
```

**Pattern**: Widgets request redraws via channels instead of polling.

---

## Custom Terminal Implementation

Codex uses a **CustomTerminal** (forked from ratatui::Terminal) to support inline viewport:

```rust
pub type Terminal = CustomTerminal<CrosstermBackend<Stdout>>;

pub struct Tui {
    terminal: Terminal,
    pending_history_lines: Vec<Line<'static>>,
    alt_saved_viewport: Option<Rect>,  // Saved inline viewport when entering alt screen
    alt_screen_active: Arc<AtomicBool>,  // Track current screen mode
    terminal_focused: Arc<AtomicBool>,   // Track window focus
    // ...
}
```

**Why custom?**
- Standard ratatui::Terminal assumes alternate screen
- Custom terminal manages inline viewport dynamically
- Preserves scrollback history
- Allows toggling between inline ↔ alternate screen

---

## Application Event Model

**Custom AppEvent enum** routes all application events:

```rust
pub enum AppEvent {
    CodexEvent(Event),           // Agent/protocol events
    NewSession,                  // User starts new session
    ExitRequest,                 // User wants to quit
    CodexOp(Op),                 // Forward operation to agent
    StartFileSearch(String),     // Begin file search
    FileSearchResult { ... },    // Search results ready
    InsertHistoryCell(Box<dyn HistoryCell>),  // Add to history
    UpdateModel(String),         // Switch LLM model
    // ... 30+ event types
}
```

**Pattern**: Central event loop dispatches to handlers.

```rust
loop {
    select! {
        Some(tui_event) = tui.event_stream().next() => {
            let app_events = app.handle_tui_event(tui_event);
            for event in app_events {
                // Dispatch AppEvent to appropriate handler
            }
        }
        Some(agent_event) = agent.event_stream().next() => {
            let app_event = AppEvent::CodexEvent(agent_event);
            // Process agent event
        }
    }
}
```

---

## Interruption Handling

**ESC key interruption** (likely pattern, based on event stream):

```rust
match tui_event {
    TuiEvent::Key(key) if key.code == KeyCode::Esc => {
        // Set interruption flag
        is_interrupted.store(true, Ordering::Relaxed);
        // Agent loop checks this flag and stops streaming
    }
    _ => {}
}
```

**Agent loop**:
```rust
while let Some(chunk) = agent_stream.next().await {
    if is_interrupted.load(Ordering::Relaxed) {
        break;  // Stop streaming, preserve partial results
    }
    // Process chunk
}
```

---

## Key Takeaways for Codelet

### What to Copy

1. **Stack**: ratatui + crossterm + tokio::select!
2. **Mode**: Inline viewport by default (NO alternate screen initially)
3. **Event architecture**: async_stream::stream! with tokio::select!
4. **Panic hook**: Terminal restoration on crash
5. **Keyboard enhancements**: Disambiguate ESC codes, gracefully degrade
6. **Focus tracking**: Pause animations when unfocused
7. **Custom AppEvent enum**: Central event routing

### What to Simplify (Initially)

1. **Skip alternate screen toggle** - Just inline mode for v1
2. **Skip Unix suspend handling** - Add later if needed
3. **Skip frame scheduling** - Simpler redraw model initially
4. **Minimal custom terminal** - May be able to use standard ratatui::Terminal with inline config

### Critical Differences from TypeScript Codelet

| Aspect | TypeScript (codelet) | Rust (codex) |
|--------|---------------------|-------------|
| **TUI framework** | None (raw Node.js readline + process.stdin) | ratatui (full TUI widgets) |
| **Event model** | EventEmitter (sync) | async_stream + tokio::select! |
| **Raw mode** | process.stdin.setRawMode(true/false) | crossterm::enable_raw_mode() |
| **Cleanup** | process.on('exit') hook | Panic hook + Drop trait |
| **Input queue** | Manual array + EventEmitter | tokio::sync::mpsc channels |
| **Status display** | setInterval + console.log | ratatui widgets + FrameRequester |

---

## Implementation Plan for Codelet

### Phase 1: Minimal TUI (MVP)

```rust
// Dependencies
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
async-stream = "0.3"

// Structure
pub struct InteractiveTui {
    terminal: Terminal,  // ratatui::Terminal with CrosstermBackend
    event_stream: Pin<Box<dyn Stream<Item = TuiEvent>>>,
    is_interrupted: Arc<AtomicBool>,
}

// Event loop
loop {
    select! {
        Some(tui_event) = tui.event_stream.next() => {
            match tui_event {
                TuiEvent::Key(key) if key.code == KeyCode::Esc => {
                    // Interrupt agent
                }
                TuiEvent::Key(key) => {
                    // Process input
                }
                TuiEvent::Draw => {
                    // Redraw UI
                }
            }
        }
        Some(agent_chunk) = agent.stream.next() => {
            if !is_interrupted.load(Ordering::Relaxed) {
                // Display agent output
            }
        }
    }
}
```

### Phase 2: Enhanced Features

- Status display with elapsed time (ratatui widgets)
- Input queue with channel-based persistence
- Provider switching UI
- Startup card rendering

### Phase 3: Advanced (Optional)

- Alternate screen toggle
- Unix suspend/resume
- Frame scheduling
- Focus-aware animations

---

## Conclusion

Codex demonstrates that **ratatui works beautifully in inline mode** with preserved scrollback. The async event stream architecture with `tokio::select!` elegantly solves the interruption problem while maintaining responsive UI.

For codelet, we should **adopt the full codex stack** rather than trying to reinvent with lower-level crossterm-only approach. This gives us:
- ✅ Battle-tested architecture
- ✅ Rich UI capabilities (status display, formatting)
- ✅ Inline mode with scrollback preservation
- ✅ Clean interruption via ESC key
- ✅ Proper terminal restoration
- ✅ Future-proof for UI enhancements

The complexity is justified by the robustness and extensibility gained.
