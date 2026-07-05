# AST Research — BLOCK-011 mouse-wheel scroll support

## Goal
Confirm the `handle_mouse` seam pattern to mirror for the blocklist view, and
the WheelVelocity field/derive constraints.

## Existing `handle_mouse` implementations (AstGrep: `pub fn handle_mouse(&mut self, ev: MouseEvent) -> $RET { $$$BODY }`)

| File | Line | Signature |
|------|------|-----------|
| `views/provider_settings/mouse.rs` | 22 | `pub fn handle_mouse(&mut self, ev: MouseEvent) -> ProviderSettingsEvent` |
| `views/changed_files/mouse.rs` | 20 | `pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent` |
| `views/checkpoints/keys.rs` | 57 | `pub(super) fn handle_mouse(...) -> CheckpointsEvent` |
| `views/model_selector/dispatch.rs` | 190 | `pub fn handle_mouse(&mut self, ev: MouseEvent) -> ModelSelectorEvent` (verified by Read) |

**Blocklist has NO `handle_mouse` method** — it is the outlier.

## Canonical wheel pattern (model_selector/dispatch.rs:190-207, verified)
```rust
let dir = match ev.kind {
    MouseEventKind::ScrollUp => WheelDirection::Up,
    MouseEventKind::ScrollDown => WheelDirection::Down,
    _ => return ModelSelectorEvent::Ignored,
};
let step = self.wheel.step(dir);          // wheel: WheelVelocity field
let mover = match dir { Up => Self::move_up, Down => Self::move_down };
for _ in 0..step.unsigned_abs() { mover(self); }
ModelSelectorEvent::Consumed
```
provider_settings/mouse.rs:22-38 is the same shape (uses `move_clamped(step)`).

## Navigator routing (navigator_events.rs, verified)
- `handle_model_selector_event` (94-102) routes `Event::Mouse` into
  `handle_mouse` BEFORE the `let Event::Key(key) = event else {...}` guard.
- `handle_blocklist_event` (148-168) has NO such branch — line 149 drops
  `Event::Mouse` via `let Event::Key(key) = event else { return ignored() }`.

## WheelVelocity constraints (components/scroll_viewport.rs, verified)
- `pub struct WheelVelocity { last: Cell<Option<Instant>>, velocity: Cell<u32> }`
  (lines 80-83). Has a manual `impl Default` (85-89). `step(&self, dir) -> i32`
  (104) — takes `&self` (interior mutability via Cell).
- `WheelVelocity` is **NOT** `#[derive(Debug, Clone)]`.
- `WheelDirection` (69-73) derives `Debug, Clone, Copy, PartialEq, Eq`.

### ⚠️ Derive conflict (KEY IMPLEMENTATION DECISION)
`BlocklistView` currently `#[derive(Debug, Clone, Default)]` (mod.rs:68). Adding
a `wheel: WheelVelocity` field will BREAK the `Debug`/`Clone` derives because
`WheelVelocity` implements neither. `ModelSelectorView` avoids this by being a
plain struct with a MANUAL `impl Default` and no `Clone`/`Debug` derive.

Two viable fixes (implementer picks the one that compiles cleanest, least blast):
1. **Additive (preferred):** add `#[derive(Debug, Clone)]` to `WheelVelocity` in
   `scroll_viewport.rs`. Its `Cell<Option<Instant>>` / `Cell<u32>` fields are both
   `Clone` and `Debug` (inner types are `Copy`), so this is safe + behavior-neutral.
2. **Local:** replace `BlocklistView`'s `#[derive(...)]` with manual `Default`
   (mirror ModelSelectorView) and drop `Clone`/`Debug` if unused. `Navigator`
   (navigator.rs:55) is a plain struct and does NOT require BlocklistView: Debug/Clone,
   so dropping them is feasible — but verify nothing in tests clones/debugs the view.

## Conclusion
Add `wheel: WheelVelocity` + `handle_mouse` to BlocklistView mirroring
model_selector (move_up/move_down already `adjust_scroll()`), and add a mouse
branch to `handle_blocklist_event`. Resolve the derive conflict via option 1
(preferred) or 2. Scope: `blocklist/mod.rs`, `navigator_events.rs`, and possibly
`scroll_viewport.rs` (1 line).
