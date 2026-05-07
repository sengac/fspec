# 01 — Executive Summary

## Verdict

**The port is feasible.** No single ratatui crate is a drop-in replacement
for `VirtualList` or `Dialog`, but the ratatui ecosystem covers ~70 % of
`VirtualList` and ~90 % of `Dialog` rendering out of the box. The
*behavioural glue* (priority-based input routing, dynamic Yoga-style flex
measurement, group-id selection preservation, mouse-tracking toggle for
native text selection, scroll-stick-with-user-detection) is custom in
**every** comparable project we surveyed — including OpenAI's codex.

> **Strategy: compose 3–4 official-ish crates + ~600 LoC of custom Rust per
> anchor component, on top of a Helix-style `Compositor` for input
> priority.**

## Headline recommendations

1. **Adopt the ratatui-org [`templates/component`](https://github.com/ratatui/templates/tree/main/component)
   architecture** (Component trait + `mpsc<Action>` bus + `tokio::select!`
   over events / tick / render / cancel) as the app shell.
2. **Build a Helix-inspired `Compositor`** (`Vec<Box<dyn Component>>`,
   walked top-down, each returning `EventResult::{Consumed, Ignored}`) as
   the input-priority manager. This is **exactly** the contract our
   `InputPriority` enum + `useInputCompat` registry implements today.
   ~30 LoC of Rust.
3. **Use [`tui-popup`](https://crates.io/crates/tui-popup)** (now in the
   official `ratatui-org/tui-widgets` mono-repo) for `Dialog` rendering.
4. **Use [`tui-widget-list`](https://crates.io/crates/tui-widget-list)**
   (preiter93) as the base for `VirtualList`; bolt on a custom `ListItems`
   trait for our lazy-mode `getItems(start, end)` accessor and a custom
   group-id preservation layer. Use ratatui core's
   [`Scrollbar`](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Scrollbar.html)
   widget.
5. **Use [`tui-textarea`](https://github.com/rhysd/tui-textarea)** for
   `MultiLineInput`. It ships a rope buffer and a documented vim-mode
   example that shortcuts most of our state machine.
6. **Drop our custom SGR mouse parser** — crossterm parses SGR 1006
   internally and delivers structured `MouseEvent` values, eliminating
   `src/tui/utils/mouseProtocol.ts` entirely (≈70 LoC saved).
7. **Replace Yoga `measureElement` with ratatui's
   `Constraint::*`-based layout.** The `heightAdjustment: -1/-2` workaround
   for Yoga measurement quirks simply disappears.
8. **Optionally consider [`rat-event` / `rat-salsa`](https://github.com/thscharler/rat-salsa)**
   for input dispatch — it has an explicit `Dialog` qualifier that maps
   1:1 to our `InputPriority::CRITICAL`. Recommended *only* if we want
   external maintenance of that subsystem; otherwise the ~30-LoC homegrown
   Compositor is leaner.

## What is **not** in any crate (must be hand-written)

| Behaviour | Origin | Approx LoC |
|---|---|---|
| Priority-based input dispatcher (5 levels) | `src/tui/input/InputManager.tsx` | ~50 LoC Rust |
| Group-id selection preservation across mutations | `VirtualList.tsx` | ~30 LoC |
| Scroll vs Item selection modes | `VirtualList.tsx` | ~20 LoC |
| `scrollToEnd` auto-stick + user-scrolled-away | `VirtualList.tsx` | ~15 LoC |
| Mouse-tracking 5 s debounce for native text selection | `VirtualList.tsx` | ~40 LoC + tokio timer |
| Mouse-wheel velocity acceleration | `VirtualList.tsx` | ~20 LoC |
| Lazy-mode `itemCount + getItems(start, end)` | `VirtualList.tsx` | ~50 LoC trait |
| Draggable scrollbar thumb hit-testing | (planned but not in TS) | ~30 LoC |
| **Total custom Rust** | | **~255 LoC** |

## What goes away (no longer needed in Rust)

| Removed | Reason |
|---|---|
| `useLayoutEffect` race-fix in `useInputCompat` | Rust registration is synchronous |
| `MOUSE_ENABLE` / `MOUSE_DISABLE` raw escape writes | crossterm `EnableMouseCapture` |
| Custom SGR 1006 parser | crossterm parses it internally |
| Yoga `measureElement` setTimeout-0 trick | ratatui `Layout::vertical([Constraint::*])` |
| `heightAdjustment: -1/-2` Yoga workaround | No measurement quirks to compensate |
| `setTimeout(…, 0)` Yoga-layout-complete hack | No reconciliation phase |
| Scrollbar string memoisation cache | ratatui core `Scrollbar` is allocation-free |
| `useId()` instance keying | Each component owns its own state |

## Key surprise: Codex is **not** the model to copy

OpenAI's Codex CLI does **not** virtualize chat in ratatui. It writes
finalised cells to the host terminal's native scrollback via
DECSTBM + `RI` ANSI sequences and only paints the bottom "live" region.
That works because Codex is a one-stream chat REPL. fspec is a
multi-pane Kanban / agent / file-search application — we genuinely need
**alt-screen mode + virtualised lists**, which means our reference
templates are Helix and lazygit, not Codex.

## Approximate effort

* **VirtualList port:** ~600 LoC Rust (vs 689 LoC TS), ~5–8 days.
* **Dialog port:** ~80 LoC + `tui-popup`, ~1 day.
* **Input priority Compositor:** ~50 LoC + tests, ~1–2 days.
* **MultiLineInput on `tui-textarea`:** ~200 LoC glue, ~3–5 days.
* **Mouse subsystem (incl. text-selection toggle):** ~100 LoC, ~1 day.
* **App shell (Component trait + mpsc bus + tokio loop):** copy from
  ratatui-org template, ~2 days.

These estimates are foundational components only — they do **not** include
the migration of every consumer (BoardView, AgentView, CheckpointViewer,
FileSearchPopup, SlashCommandPalette, dozens of dialogs).

## Open decisions before ACDD on RPC-002 begins

See [`11-open-questions-and-risks.md`](11-open-questions-and-risks.md).

The four decisions that change the breakdown:
1. Alt-screen vs inline-with-scrollback rendering mode.
2. Adopt `rat-event` / `rat-salsa` framework, or roll a 30-LoC Compositor?
3. Adopt `tui-realm` for React-like ergonomics, or stay closer to bare
   ratatui? (Recommendation: bare ratatui — `tui-realm`'s focus model
   fights our priority model.)
4. Implement `MultiLineInput` on `tui-textarea`, or rebuild from scratch?
