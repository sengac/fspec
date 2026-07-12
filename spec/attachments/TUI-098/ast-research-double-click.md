# AST Research: Double-click detection for TUI-098

## Problem Statement

The `/resume` session picker in the Rust TUI currently only supports single-click to move selection. The user must press Enter to resume. We need to add double-click detection so that double-clicking a session row immediately resumes it.

## Current Code Paths

### 1. ResumeSessionView::handle_mouse() (resume_session_view.rs:135-167)

```rust
pub fn handle_mouse(
    &mut self, ev: MouseEvent, body_rect: Rect, visible_rows: usize,
) -> ResumeSessionViewOutcome {
    // ... hit-test ...
    match ev.kind {
        MouseEventKind::ScrollUp => { ... }
        MouseEventKind::ScrollDown => { ... }
        MouseEventKind::Down(MouseButton::Left) => {
            let candidate = self.scroll_offset + (ev.row - body_rect.y) as usize;
            if candidate < self.sessions.len() {
                self.selected_index = candidate;
                self.adjust_scroll(visible_rows);
                ResumeSessionViewOutcome::Continued  // ← Just moves selection
            }
        }
        _ => ResumeSessionViewOutcome::Ignored,
    }
}
```

**Key observation:** `MouseEventKind::Down(MouseButton::Left)` currently always returns `Continued`. We need to detect if this is a double-click and return `Selected(SessionId)` instead.

### 2. mouse_dispatch.rs:25-43 — Routing

```rust
pub(super) fn handle_mode_view_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
    // ...
    if let Some(view) = self.resume_view.as_mut() {
        match view.handle_mouse(ev, body_rect, visible_rows) {
            ResumeSessionViewOutcome::Selected(session_id) => {
                self.resume_view = None;
                self.emit(Action::AttachToSession(session_id));
                return Some(EventResult::consumed());
            }
            _ => return Some(EventResult::consumed()),
        }
    }
    // ...
}
```

**Key observation:** The routing already handles `Selected` outcome correctly. We just need `handle_mouse()` to return `Selected` on double-click.

### 3. ResumeSessionView struct (resume_session_view.rs:49-55)

```rust
pub struct ResumeSessionView {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    scroll_offset: usize,
    delete_confirm: Option<ConfirmDialog>,
    wheel: WheelVelocity,
}
```

**Key observation:** We need to add fields to track the last click for double-click detection.

## Implementation Plan

### A. Add DoubleClickDetector to ResumeSessionView

```rust
struct DoubleClickDetector {
    last_click_row: Option<usize>,
    last_click_time: Option<Instant>,
    timeout: Duration, // 300ms
}
```

### B. Modify handle_mouse() MouseEventKind::Down branch

```rust
MouseEventKind::Down(MouseButton::Left) => {
    let candidate = self.scroll_offset + (ev.row - body_rect.y) as usize;
    if candidate < self.sessions.len() {
        let now = Instant::now();
        if let Some(detector) = &mut self.double_click {
            if detector.is_double_click(candidate, now) {
                // Double-click on same row within timeout
                let info = &self.sessions[candidate];
                return ResumeSessionViewOutcome::Selected(SessionId::new(info.id.clone()));
            }
        }
        // Single-click: move selection
        self.selected_index = candidate;
        self.adjust_scroll(visible_rows);
        ResumeSessionViewOutcome::Continued
    }
}
```

### C. Update footer hint text

In `resume_session_view.rs:268`, change:
```rust
"Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
→ "DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
```

### D. Test file

Add tests to `codelet/fspec-tui/tests/rpc028_popup_scroll.rs`:
- `tui098_double_click_same_row_resumes_session()` — two clicks within 300ms → Selected
- `tui098_two_clicks_over_300ms_are_single_clicks()` — two clicks 500ms apart → Continued
- `tui098_quick_clicks_different_rows_are_single_clicks()` — clicks on different rows → Continued

## Files to Modify

1. `codelet/fspec-tui/src/views/agent/resume_session_view.rs` — Add DoubleClickDetector, modify handle_mouse()
2. `codelet/fspec-tui/src/views/agent/mode_view_render.rs` — Update footer hint text
3. `codelet/fspec-tui/tests/rpc028_popup_scroll.rs` — Add double-click tests

## Dependencies

- `std::time::Instant` for timing
- `std::time::Duration` for timeout
- No new crate dependencies needed

## Integration Points

- The `ResumeSessionViewOutcome::Selected` outcome is already routed in `mouse_dispatch.rs:36-40`
- The `Action::AttachToSession` handler in `dispatch_resume_search_views.rs` handles the actual resume
- No changes needed to the App-level handler — the outcome routing is already correct
