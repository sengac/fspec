# RPC-094 — Scrollback scroll surface port spec

## Goal

Bring the Rust `ScrollbackList` (in `codelet/fspec-tui/src/views/agent/scrollback.rs`)
to mouse-wheel + line-scroll parity with `src/tui/components/VirtualList.tsx`
as consumed by `src/tui/components/AgentView.tsx`. The TS reference
behaviour is anchored at `AgentView.tsx:4373` (arrow-line forwarding) and
`AgentView.tsx:4435-4458` (1×–5× wheel velocity ramp).

## Scope

IN scope (this card):
- Mouse wheel ScrollUp/ScrollDown over the scrollback rect.
- 1×–5× wheel velocity ramp shared via existing
  `components/scroll_viewport.rs::WheelVelocity` (RPC-028 primitive,
  unused by scrollback today).
- Up/Down arrow line-scroll when MultiLineInput cursor is at the
  first / last visual line (the keys would otherwise be swallowed).
- Home jumps scrollback offset to 0 (drops stick mode).
- 1-cell ratatui `Scrollbar` widget gutter on the right edge of the
  scrollback area when `total_visual_rows > viewport_height`.

OUT of scope (each gets its own future card):
- Full generic `VirtualList<T>` per RPC-002 §8.
- CheckpointViewer / TurnContentModal / FileDiffViewer ports.
- Group-based selection, lazy mode, SelectionMode::Item.

## Architecture

### New primitives

- Reuse `WheelVelocity` from `components/scroll_viewport.rs` — no new
  file required for velocity logic.
- `ScrollbackList` gains `last_rect: Option<Rect>` (analogous to
  `AgentView::last_render_area`) so mouse hit-testing can be done
  in `views/agent/mouse_dispatch.rs` without leaking layout knowledge
  out of `agent.rs`.

### Edited files

| File | Why | Expected delta |
|---|---|---|
| `codelet/fspec-tui/src/views/agent/scrollback.rs` | Add `scroll_lines(dir, amount)` + `last_rect` + scrollbar render | +~40 LoC; stay <300 |
| `codelet/fspec-tui/src/views/agent/mouse_dispatch.rs` | Route wheel events to scrollback after popups/mode-views ignore | +~30 LoC; stay <300 |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | Up/Down passthrough only when input ignored; emit new actions | +~25 LoC; stay <300 (currently 295) |
| `codelet/fspec-tui/src/views/agent/multiline_input.rs` | Expose `cursor_at_top()` / `cursor_at_bottom()` if not already | unknown — investigate |
| `codelet/fspec-tui/src/components/mod.rs` | Add `Action::ScrollbackLineUp`, `Action::ScrollbackLineDown`, `Action::ScrollbackHome`, `Action::ScrollbackMouseWheelUp(u32)`, `Action::ScrollbackMouseWheelDown(u32)` | +5 variants |
| `codelet/fspec-tui/src/app/dispatch.rs` | Wire the 5 new actions to `ScrollbackList` | +~20 LoC |
| `codelet/fspec-tui/src/views/agent.rs` | Allocate 1-cell gutter when scrollbar visible; cache `last_rect` into the scrollback before rendering | +~10 LoC; stay <300 |

### Source-shape invariants

- Every touched file stays under 300 LoC.
- No new `unwrap`, no new `panic!`.
- No new dependencies (ratatui `Scrollbar` widget ships with ratatui core).

## TS reference walk

### Wheel velocity (`AgentView.tsx:4435-4458`)

```ts
// SGR mouse: button byte 96 = wheel up, 97 = wheel down.
const now = Date.now();
if (now - lastScrollTime < 150) {
  scrollVelocity = Math.min(scrollVelocity + 1, 5);
} else {
  scrollVelocity = 1;
}
lastScrollTime = now;
const amount = scrollVelocity;
if (button === 96) scrollOffset = Math.max(0, scrollOffset - amount);
else if (button === 97) scrollOffset = Math.min(maxOffset, scrollOffset + amount);
```

→ exact match to `WheelVelocity::step` in `scroll_viewport.rs` (which
was written for popups in RPC-028 and is reusable verbatim).

### Arrow line forwarding (`AgentView.tsx:4373`)

```ts
// If MultiLineInput did NOT consume the arrow (cursor at first/last
// visual line), forward to scrollback as a 1-line scroll.
if (!inputConsumed && key === 'up') scrollOffset = Math.max(0, scrollOffset - 1);
if (!inputConsumed && key === 'down') scrollOffset = Math.min(maxOffset, scrollOffset + 1);
```

→ The Rust `MultiLineInput` already returns
`InputEventOutcome::Ignored` when it can't handle the key. The
`handle_event` orchestrator in `dispatch.rs` currently throws those
ignored Up/Down keys away. RPC-094 intercepts them BEFORE forwarding
to `self.input.handle_event` if and only if the cursor is at the
relevant edge — otherwise let the input keep them.

## Test plan (`spec/features/rpc094-agentview-scrollback-scroll.feature`)

Each scenario maps to exactly one Gherkin scenario with `@step`
comments in `codelet/fspec-tui/tests/scrollback_scroll_rpc094.rs`.

| # | Scenario | Type |
|---|---|---|
| 1 | Mouse wheel up inside scrollback rect scrolls by velocity, drops stick | unit |
| 2 | Mouse wheel down inside scrollback rect scrolls by velocity, re-enters stick at tail | unit |
| 3 | Wheel velocity ramps 1→5 within 150 ms | unit |
| 4 | Wheel velocity resets to 1 after 150 ms gap | unit |
| 5 | Mouse wheel up over header / footer / input rect is ignored by scrollback | unit |
| 6 | Up arrow at MultiLineInput first visual line scrolls scrollback by 1 line | integration |
| 7 | Down arrow at MultiLineInput last visual line scrolls scrollback by 1 line | integration |
| 8 | Up arrow mid-buffer stays inside MultiLineInput (no scrollback change) | integration |
| 9 | Home key (when input does not consume it) jumps offset to 0 + drops stick | unit |
| 10 | Scrollbar gutter renders when total_visual_rows > viewport_height | snapshot |
| 11 | Scrollbar gutter hidden when total <= viewport | snapshot |
| 12 | Popup open absorbs wheel; scrollback offset unchanged | integration |
| 13 | Source shape: every touched file under 300 LoC | source-shape |

Estimate: **5 points** (≈3 hours). Mostly wiring + 1 new shared
`Scrollbar` widget call; the hard work (`WheelVelocity`, viewport
math, ScrollState) already exists.

## Acceptance

User runs the Rust TUI, types enough chat messages to fill > viewport,
then:
- Scrolls the trackpad up over the scrollback area → visible content
  scrolls up; thumb moves up.
- Scrolls back down to the bottom → stick-to-bottom re-engages and
  new chunks auto-scroll.
- Holds Up arrow while the input is empty (cursor at last line which
  is also first) → scrollback scrolls one line at a time.
- Opens `/help` popup → trackpad inside popup scrolls the popup,
  trackpad outside popup leaves scrollback untouched (RPC-028
  invariant preserved).
