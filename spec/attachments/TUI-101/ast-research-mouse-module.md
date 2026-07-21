# AST Research: Mouse Module Structure and Scrollbar Patterns

## Mouse Module Files

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 34 | Module entry, re-exports |
| `hit_test.rs` | 102 | `rect_contains()` half-open hit-test |
| `toggle.rs` | 128 | Debounced mouse-tracking toggle |
| `gesture.rs` | 281 | SelectionRecognizer state machine |
| `selection.rs` | 263 | Cell, Selection, RowSpan geometry |
| `clipboard.rs` | 135 | OSC 52 clipboard writer |

## Scrollbar Math (list_scrollbar.rs)

```rust
thumb_h = ((visible * h) / total).max(1)
thumb_pos = (scroll_offset * h) / total
```

## Source-Shape Constraint

Every file must be < 300 lines. Enforced by source_shape tests across all modules.

## Integration Pattern

Mouse dispatch follows layered hit-testing: mode views → popups → turn modal → scrollback → composer. Each layer hit-tests against cached rects from render.
