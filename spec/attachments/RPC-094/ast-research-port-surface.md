# RPC-094 — AST research: existing port surface to extend

Generated via `AstGrep` over the Rust port. Captures the existing
public methods we will compose or extend.

## ScrollbackList (`codelet/fspec-tui/src/views/agent/scrollback.rs`)

Existing `pub fn` surface:

| Signature | Role |
|---|---|
| `push(&mut self, chunk: RenderedChunk)` | append + rewrap + re-anchor |
| `insert(&mut self, idx, chunk)` | RPC-093 splice-in for thinking |
| `rewrap_at(&mut self, i: usize)` | RPC-091 in-place rewrap |
| `set_viewport_height(&mut self, h: u16)` | refresh cached height |
| `set_viewport_width(&mut self, w: u16)` | refresh cached width + rewrap all |
| `scroll_up(&mut self, lines: usize)` | drop stick, decrement offset |
| `scroll_down(&mut self, lines: usize)` | inc offset, re-stick at tail |

Reads: `chunks()`, `chunks_mut()`, `chunk_count()`, `scroll_state()`,
`visible_window(viewport_lines)`, `total_visual_rows()`.

Renders: `render_count_visited(area, buf) -> usize`,
plus `impl Widget for &mut ScrollbackList`.

**RPC-094 will add:** `last_rect: Option<Rect>` (set inside
`render_count_visited`), `scroll_lines(WheelDirection, u32)` thin
wrapper over `scroll_up`/`scroll_down`, `jump_to_top()` already exists
(used for Home), and the scrollbar painting block inside
`render_count_visited` when `total_visual_rows > area.height`.

## WheelVelocity (`codelet/fspec-tui/src/components/scroll_viewport.rs`)

Existing `pub fn` surface:

| Signature | Role |
|---|---|
| `step(&self, dir: WheelDirection) -> i32` | returns signed magnitude |
| `step_at(&self, dir: WheelDirection, now: Instant) -> i32` | test-friendly variant |

Verified: this primitive ALREADY matches `AgentView.tsx:4435-4458`
1×–5× ramp with 150 ms gap reset. RPC-094 will instantiate one
`WheelVelocity` field on `AgentView` (presentation owns the cadence
state) and call `step()` from the new `handle_scrollback_mouse`.

## MultiLineInput (`codelet/fspec-tui/src/views/agent/multiline_input.rs`)

`handle_key` already returns `InputEventOutcome::Ignored` for Up/Down
when the cursor is on the first/last visual line (lines 159-167 of
multiline_input.rs). This means the existing pipeline already gives
us the hook — `dispatch.rs` just needs to convert the `Ignored`
outcome from arrow keys into a scrollback line-scroll Action instead
of bubbling further.

**No public surface changes required on MultiLineInput.** The "cursor
at edge" semantics are already encoded in the `Ignored` outcome.

## Action enum (`codelet/fspec-tui/src/components/mod.rs`)

Existing relevant variants:

- `ScrollbackPageUp` (RPC-019/024)
- `ScrollbackPageDown` (RPC-019/024)

**RPC-094 will add:**

- `ScrollbackLineUp` — emitted by arrow-up at input edge
- `ScrollbackLineDown` — emitted by arrow-down at input edge
- `ScrollbackHome` — emitted by Home key
- `ScrollbackMouseWheelUp(u32)` — carries velocity multiplier
- `ScrollbackMouseWheelDown(u32)` — carries velocity multiplier

Each variant lands in `App::dispatch` (codelet/fspec-tui/src/app/dispatch.rs)
next to the existing `ScrollbackPageUp/PageDown` arms, calling
`ctx.scrollback.scroll_up(n)` / `scroll_down(n)` / `jump_to_top()` on
the current `SessionContext`.

## Routing summary (post-RPC-094)

```
Event::Key
  ├─ Ctrl+R                        → OpenSearchView
  ├─ mode_view consumes             → ConsumedByModeView
  ├─ popup consumes                 → ConsumedByPopup
  ├─ Esc / Ctrl+C                  → AgentEscPressed / Interrupt
  ├─ PageUp                        → Action::ScrollbackPageUp
  ├─ PageDown / End                → Action::ScrollbackPageDown
  ├─ Home (NEW)                    → Action::ScrollbackHome  (when input ignores)
  ├─ Shift+Arrow                   → History*/Session*
  ├─ Up/Down at input edge (NEW)   → Action::ScrollbackLineUp/Down
  └─ otherwise                     → MultiLineInput.handle_event

Event::Mouse
  ├─ mode_view consumes             → ConsumedByModeView
  ├─ popup consumes                 → ConsumedByPopup
  ├─ wheel inside scrollback rect (NEW) → Action::ScrollbackMouseWheel*(velocity)
  └─ otherwise                     → EventResult::ignored
```
