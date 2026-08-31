# Multiplexing (Mux Mode)

Mux mode splits the TUI screen into a configurable grid of top-level views — the Board, agent sessions, Changed Files, and Git Checkpoints — so you can monitor the factory floor and every agent side-by-side instead of switching views.

```
┌───────────────────────┬────────────────────────────┐
│  Board               │  Agent 1                    │
│  (focused, ●)        │                             │
│  BACKLOG │ SPEC │ …  │  …scrollback…               │
│                      │  [live input composer]      │
└───────────────────────┴────────────────────────────┘
 MUX 2 panes [Board|Agent]  ●pane 0  /mux config · Shift+←/→ focus · drag divider
```

The grid is driven by a single `MuxConfig` and a live `MultiplexLayout` (in `rust/fspec-tui/src/views/multiplex/`). This document covers the configuration model, layout math, input routing ("the trap"), multi-agent panes, and persistence.

## Configuring the grid

Everything is driven by the `/mux` slash command:

```
/mux                  → open the MuxConfigDialog (interactive config)
/mux on               → enable with the saved/default config
/mux off              → disable, return to the view that was active before mux
/mux h | /mux v       → horizontal (side-by-side) or vertical (stacked)
/mux 2..=4            → set the pane count with the default pane kinds
/mux board agent 40   → explicit pane list + first split percent
/mux save             → persist the current config to disk
/mux default          → reset to the default preset
/mux help             → print the subcommand reference
```

Parse errors (`/mux board zzz`, `/mux 5`, `/mux board agent 5`) surface as a one-line notice in the agent scrollback and leave the current grid untouched.

The **MuxConfigDialog** (bare `/mux`, or picking `/mux` from the slash popup) shows rows for **Enabled**, **Orientation**, and one row per pane:

- `↑↓` move the field cursor (wraps); `←→` cycle the highlighted row's value
- `a` appends a pane row (kind Board, max 4 panes); `Backspace` removes the highlighted one (min 2)
- `Enter` applies the draft; `s` applies **and** persists; `Esc` cancels without applying

The dialog edits a **draft copy** of the live config — the grid underneath stays exactly as it is until you commit. When the draft's pane count changes, the split scale is re-derived for the new count (existing positions are preserved proportionally; equal splits stay equal). Split percentages themselves are divider-drag-driven only and are not hand-editable in the dialog.

The default preset is horizontal **Board | Agent** at 50/50. A fresh entry focuses the **Board** pane (the view you came from); the agent pane is the persisted "home" focus restored when a saved config loads.

## The configuration model

```rust
pub struct MuxConfig {
    pub orientation: MuxOrientation,      // Horizontal | Vertical
    pub splits: Vec<u16>,                 // n panes → n-1 percentage entries
    pub panes: Vec<MuxPaneKind>,          // Board | Agent | ChangedFiles | Checkpoints
    pub focused_pane: usize,
    pub enabled: bool,
}
```

**The percentage-scale model:** `splits` holds ONE percentage per inter-pane gap — `n` panes → `n-1` entries, where `splits[i]` is pane `i`'s share of the available axis (after divider subtraction) in percent. The last pane's share is implicit: `100 − sum(splits)`, so the scale always sums to 100. A missing entry (legacy/short configs) falls back to the equal share `available/n`.

Scale operations (pure math in `multiplex/splits.rs`):

- **Equal split** — `/mux n` starts from equal portions: `[50]`, `[33, 33]`, `[25, 25, 25]`
- **Rescale on pane-count change** — adding a pane gives it an equal `100/n` share and shrinks the others proportionally; removing one re-allocates its share to the survivors (largest-remainder rounding, so the relative ratio is preserved)
- **Divider drag** — releasing a divider writes the released percent into that gap's entry; the panes to its right absorb the change proportionally, the panes to its left keep their shares
- **Normalize on load** — too-few entries rescale, sums ≥ 100 renormalize, every entry ends up in 1..=99

## Layout math

`calculate_pane_rects` (`multiplex/layout.rs`) turns the config into absolute pane `Rect`s:

- `available = terminal_axis − (n − 1)` dividers; each divider is 1 col (horizontal) or 1 row (vertical)
- Each pane gets its percent of `available`; the last pane absorbs the integer-division remainder
- No per-pane minimums — an explicit percent is honored as-is; the board view simply degrades to a blank pane below its ~64-column fit width
- A 1-cell-per-pane floor guards tiny terminals; the layout is recomputed from the live terminal area every frame, so resizes re-divide automatically

## The "trap": input routing

While mux is active, `ViewMode::Mux` owns the whole screen. `MultiplexLayout` routes every event to exactly one place:

**Keyboard — the focused pane only.** Unfocused panes receive NO keyboard events. The only mux-level keybindings are `Shift+Left` / `Shift+Right` (pane focus cycling; see [Agent window](#agent-window-multiple-sessions-in-a-grid) below). Everything else is forwarded, unchanged, to the focused pane's view handler.

**Mouse — hit-test, then forward.** Mouse events are hit-tested against the cached rects from the last render:

1. **Dividers first** (one per inter-pane gap): mouse-down starts a drag, drag moves the split live, release commits the released percent (no snap-back to equal)
2. **Panes next**: a click focuses the pane under the cursor and forwards that click to its handler; wheel/drag over a pane forwards to the currently focused pane
3. **Gaps** (footer row, outside every rect) are ignored and never move focus

**Dialogs overlay everything.** Full-screen dialogs (help, exit confirmation, create-session, the mux config dialog) still sit on the compositor above the grid and capture input, exactly as in single-view mode.

Because all views render and hit-test in absolute terminal coordinates, the focused pane gets the *original* event with zero coordinate translation — the same pattern the single-screen mode views already use.

## Agent window: multiple sessions in a grid

Agent panes are grouped in the grid and form a **window** over the ordered list of open agent sessions:

- Agent slot `i` renders the session at `window_start + i`. No two agent panes ever show the same session.
- With fewer sessions open than agent slots, only the filled slots render (no blank panes) and the other panes absorb the space. Closing a session shrinks the list; the slots stay in place and the window re-clamps.

Focus cycling with the window:

- `Shift+Left` / `Shift+Right` move pane focus one pane at a time; at the edges they **stop** (no wrap-around)
- On the **rightmost agent pane** with all agent slots filled, `Shift+Right` rotates the window forward (`[A1][A2]` → `[A2][A3]`) and `Shift+Left` rotates backward; when the window can't rotate, the keys fall through to normal focus movement
- `Shift+Right` at the rightmost pane of **any** kind (window at the end, or non-agent pane) opens the new-agent dialog (no work-unit attachment); the new session lands in the last agent slot and focus moves to it

**One live composer, many panes.** `AgentView` keeps a single `MultiLineInput`, which always holds the *focused* session's draft. The focused agent pane renders the live composer; every unfocused agent pane renders a read-only **ghost** of its own session's persisted `input_draft`, so moving focus never changes what any pane shows. When focus lands on a different agent pane, the outgoing draft is snapshotted into the old session, the incoming session's draft is restored into the live composer, and the store's current session follows the pane.

## Focus flash

When a pane becomes focused, a 350ms accent plays over it — a single full-width row of the mux footer's dark purple (RGB 74, 44, 112) sweeps from the pane's bottom edge up to the top. Backgrounds only: pane glyphs are never blanked. After the window elapses, the scan **settles** into a persistent 1-row top bar that stays on the focused pane until focus moves or mux is disabled. Every focus-change path re-arms the flash (focus cycling, click-to-focus, `Enter` on a board work unit, window rotation, new-agent focus, pane layout changes); no-op re-focuses don't. The flash is render-driven (the run loop advances the clock +16ms per frame during the 350ms window) and live-only — it is never persisted.

## Persistence

The config lives in the shared `fspec-config.json` under the `tui.mux` key — the same pattern as `tui.defaultThinkingLevel`:

- **User scope**: `~/.fspec/fspec-config.json`; a project-level `spec/fspec-config.json` overrides via deep merge
- **Missing or malformed key** → the default preset (traced, never fatal)
- Loaded at bootstrap; auto-saved on `/mux save`, on committing the config dialog with `s`, and on mux exit
- Persisted shape: orientation, the full `n−1` entry split scale, pane list, focused pane, and the enabled flag

## When mux is off

With mux disabled the TUI behaves byte-for-byte as before: single full-screen view, no footer bar, no dividers, no flash, no routing layer. Entering and leaving mux never mutates the board or agent store state.
