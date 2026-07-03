# AST Research — COPY-003 Mouse gesture recognizer

## Goal
Confirm the crossterm MouseEvent shape, the debounce/timer pattern to mirror, and the render-tick arm the recognizer's `tick(now)` hooks into.

## crossterm MouseEvent / MouseEventKind
crossterm exposes `Event::Mouse(MouseEvent { kind, column, row, modifiers })`. `MouseEventKind` variants used: `Down(MouseButton::Left)`, `Drag(MouseButton::Left)`, `Up(MouseButton::Left)`, `ScrollUp`, `ScrollDown`. The recognizer maps `column`/`row` into a `Cell { row, col }` (note: crossterm is column,row → Cell{row,col}).

## Debounce-timer pattern to mirror
`codelet/fspec-tui/src/mouse/toggle.rs` uses `std::time::Duration` (line 25) + a re-enable timer. COPY-003 mirrors the "press-hold detection" idea but is PURE (no tokio spawn): it records `Pressed { cell, at: Instant }` and compares an injected `now: Instant` against `at + HOLD` inside `tick(now)`. `HOLD = Duration::from_millis(400)`.

## Cell reuse (COPY-002)
`Cell` now exists at `crate::mouse::selection::Cell` (COPY-002, done). The recognizer REUSES it — `SelectionGesture::Begin(Cell)` / `Extend(Cell)`. No new Cell type.

## Render tick arm (consumer context, COPY-006)
The App run loop has an existing 16ms render tick arm (app/events.rs) that COPY-006 will call `recognizer.tick(Instant::now())` from. COPY-003 itself only provides the pure `on_mouse(ev, now)` + `tick(now)` API and is unit-tested with a fake clock (base Instant + Duration offsets). No terminal, no real time.

## Module plan
- New module `codelet/fspec-tui/src/mouse/gesture.rs`, exported from mouse/mod.rs.
- `pub enum SelectionGesture { Begin(Cell), Extend(Cell), Commit, Cancel }`
- `struct SelectionRecognizer` with internal state enum `Idle | Pressed { cell: Cell, at: Instant } | Selecting`.
- `pub fn on_mouse(&mut self, ev: crossterm::event::MouseEvent, now: Instant) -> Option<SelectionGesture>`
- `pub fn tick(&mut self, now: Instant) -> Option<SelectionGesture>`
- Down(Left) → Pressed{cell, at:now}, None. First Drag(Left) → Selecting + Begin(press_cell); later Drag → Extend(cell). Up(Left) → Commit if Selecting else None (clear). tick: Pressed && now-at >= HOLD → Selecting + Begin(press_cell) once. Wheel/non-left → None.
