# TUI-101: Scrollbar Click-and-Drag Navigation Core Module

## Overview

Implement `mouse/scrollbar_drag.rs` — a shared `ScrollbarDrag` state machine that translates mouse click/drag events on proportional scrollbars into scroll offset changes.

## Architecture

### The Math

The existing scrollbar painters use this proportional formula:

```rust
// Rendering: thumb position from scroll offset
thumb_pos = (scroll_offset * area_height) / total_items
thumb_height = ((visible_items * area_height) / total_items).max(1)
```

We need the **inverse** to compute scroll offset from a click row:

```rust
// Inverse: scroll offset from click row
scroll_offset = (click_row * total_items) / area_height
```

### Track Click vs Thumb Click

- **Click on thumb**: Start dragging the thumb. The initial click sets the anchor but doesn't change the offset (the drag does).
- **Click on track (above thumb)**: Jump scroll so the clicked row becomes the top of the viewport.
- **Click on track (below thumb)**: Jump scroll so the clicked row becomes the bottom of the viewport.
- **Drag thumb**: Continuously update scroll offset as the mouse moves vertically.

### State Machine

```
Idle
  ├─ Down(Left) → Pressed { click_row, thumb_top, thumb_bottom, area_height, total_items }
  │   ├─ Drag(Left) → Dragging; emit ScrollTo(compute_offset(drag_row))
  │   └─ Up(Left) → Idle; if was_dragging, emit nothing (offset already set by drag)
  │                    if was_quick_click, emit ScrollTo(compute_offset(click_row))
  └─ (other events) → stay Idle
```

### API

```rust
pub struct ScrollbarDrag {
    // Internal state
}

impl ScrollbarDrag {
    pub fn new() -> Self;
    
    /// Feed a mouse event. Returns an optional scroll offset.
    /// None means "no action needed" (idle state, or non-left-button event).
    /// Some(offset) means "scroll to this offset".
    pub fn on_mouse(&mut self, ev: MouseEvent) -> Option<usize>;
    
    /// Reset to idle state (e.g., when content changes).
    pub fn reset(&mut self);
    
    /// True when a drag is in progress.
    pub fn is_dragging(&self) -> bool;
}
```

### Integration Contract

The `ScrollbarDrag` struct is **pure** — it knows nothing about views, actions, or stores. It only:
1. Accepts mouse events
2. Returns computed scroll offsets

The **consumer** (each view) is responsible for:
1. Hitting the scrollbar rect during render
2. Routing mouse events to the recognizer
3. Applying the returned offset to its scroll state

### Design Decisions

1. **No `SelectionRecognizer` reuse**: The scrollbar drag has different semantics (no long-press, no commit/cancel, continuous offset updates). A dedicated state machine is cleaner.

2. **Track click = page jump**: Clicking above/below the thumb jumps the viewport to that position. This matches standard scrollbar behavior in terminals and desktop applications.

3. **Thumb click without drag = single-step scroll**: If the user clicks on the thumb and releases without dragging, scroll one viewport-height in the direction of the click relative to the thumb center.

4. **Press-active flag**: Like the text selection `details_press_active` in BoardView, if the cursor strays outside the scrollbar rect during drag, the drag continues.

## File Structure

```
codelet/fspec-tui/src/mouse/scrollbar_drag.rs  (~120 LoC)
```

## Testing

Unit tests for:
- Click on track above thumb → offset jumps to top
- Click on track below thumb → offset jumps to bottom
- Click and drag thumb → continuous offset updates
- Quick click on thumb → single-step scroll
- Drag outside scrollbar area → continues to work
- Reset clears state
- Edge cases: total_items == visible (no scrollbar), area_height == 0
