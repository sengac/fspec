# Mux Mode — Design & Implementation Basis

**Feature**: `rust-mux-mode` (proposed)
**Component**: `fspec-tui`
**Status**: Design basis for ACDD work unit

## 1. Goal

Add a **multiplex ("mux") mode** to the fspec TUI that places multiple top-level
views (Board, Agent, ChangedFiles, Checkpoints) side-by-side in a configurable
grid. A `/mux` slash command toggles the mode on/off and configures the grid
breakdown (orientation, split ratios, number of panes, which view in each pane).

While mux mode is active:
- **Keyboard input** is routed to the *focused* pane only.
- **Mouse input** is hit-tested against pane rects; clicking a pane focuses it
  and the event is forwarded to that pane.
- **Shift+Left / Shift+Right** cycle pane focus (rebinding the existing
  session-cycle keys while in mux mode).
- The divider between panes is draggable (mouse) and resizable (keyboard when
  the divider has focus).
- Toggling mux off (via `/mux off` or the toggle key) returns to the current
  single-view behavior, preserving all existing state.

## 2. Current Architecture (facts from the codebase)

### 2.1 View switching today

- `rust/fspec-tui/src/views/navigator.rs` — `Navigator` owns 7 child views
  (`BoardView`, `AgentView`, `ProviderSettingsView`, `BlocklistView`,
  `ModelSelectorView`, `ChangedFilesView`, `CheckpointsView`) and a single
  `active_view: ViewMode` field.
- `Navigator::render_with_stores` renders **exactly one** child into the full
  terminal area per frame.
- `Navigator::handle_event` routes events to only the active child.
- `App::handle_event` (`app/events.rs`) runs a 4-stage cascade:
  DisconnectDialog → Compositor → Navigator → app-shortcuts, using
  `EventResult::Consumed / Ignored` (defined in `components/mod.rs`).
- Key bindings today:
  - Board → Agent: `Shift+Right` or `.` (`views/board.rs:168,268`) →
    `Action::OpenAgentView(Option<SessionId>)`.
  - Board → Agent on work unit: `Enter` (`views/board.rs:175`) →
    `Action::EnterWorkUnit(id)`.
  - Agent → Board: `Esc` → `Action::BackToBoard`.
  - Agent session cycling: `Shift+Left/Right`
    (`views/agent/dispatch.rs:24-32`) → `Action::SessionPrev/SessionNext`,
    handled in `app/dispatch_session_cycle.rs` (input-draft round-trip via
    `AgentViewStore::set_input_draft`).

### 2.2 Stores are already presentation-free

- `BoardStore` (`store/board.rs`): work units, per-column selection, scroll
  offsets, `session_attachments: HashMap<wu_id, SessionId>`.
- `AgentViewStore` (`store/agent_view.rs`): `open_sessions: Vec<SessionContext>`,
  `current_session_index`, ~20 per-session `HashMap<SessionId, _>` chrome slots.
- Views are stateless renderers borrowing these stores; all mutation flows
  through `App::dispatch` on the single App task.

### 2.3 Existing multi-pane precedent

`ChangedFilesView` (`views/changed_files/`) and `CheckpointsView`
(`views/checkpoints/`) are full-screen mode-views that already split the body
horizontally with `Constraint::Percentage(40/60)` + a 1-col divider, keep a
`focused_pane: Pane` enum, switch panes with `Tab`, and cache per-pane `Rect`s
(`last_files_rect` / `last_diff_rect`) for mouse wheel hit-testing. The mux
mode generalizes this pattern to the top-level views.

### 2.4 Constraints that matter

- **BoardView minimum width**: `views/board/grid.rs::calculate_column_widths`
  needs ≥ ~51 cols (7×8 + 6 separators + 2 borders); `views/board/render.rs:33-43`
  silently bails below that. At a 50/50 split on a 120-col terminal each pane
  is ~59 cols (fine); on a 100-col terminal ~49 cols (board would render
  blank). Mux layout must enforce a minimum pane width (default 52) and clamp
  split ratios so the board pane never drops below it.
- **AgentView is a single instance** (one `MultiLineInput`, one popup stack,
  one turn modal). Multi-session is "one view, many sessions, focus one at a
  time" (RPC-024). Mux MVP therefore shows **one live agent pane**; additional
  agent panes are out of scope for the first work unit.
- **Chunk subscriber filter**: `App::active_session_tx`
  (`watch::Sender<Option<SessionId>>`) filters chunks to the single active
  session. In mux MVP only one agent pane exists, so **no change needed** in
  the first work unit. (Multi-agent panes would require widening this to a
  `HashSet<SessionId>` — explicitly out of scope here.)
- **300-line file ceiling** (workspace rule): new code must be split across
  sibling modules.
- **ACDD is mandatory**: feature file → failing tests → implementation.
- **Workspace lint rules**: no `unwrap()`, no `panic!`, `thiserror` for public
  errors, `tracing` for logging, no `println!` in production code.

## 3. Research: What Existing Libraries Do (and Don't) Do

Cloned to `/tmp/mux-research/`: `ratatui-interact`, `rat-salsa` (rat-focus /
rat-event), `ratatui-kit`, `focusable`, `widgetui`.

**Key finding: no library "traps" input in a tmux sense.** All implement the
same pattern — a routing layer between the raw crossterm event stream and the
view handlers:

1. Split the area into pane `Rect`s.
2. Track a focused pane.
3. Keyboard: forward to the focused pane's handler only.
4. Mouse: hit-test the click against pane `Rect`s, optionally move focus to
   the clicked pane, forward the event there.

"Trapping" = **not forwarding the event to panes that aren't focused / weren't
clicked.** No OS-level capture, no virtual terminals, no subprocesses.

### 3.1 `ratatui-interact` (MIT) — most directly reusable pieces

- **`ClickRegionRegistry<T>`** (`src/traits/clickable.rs`, ~60 lines):
  register `(Rect, T)` during render; `handle_click(col, row) -> Option<&T>`
  on event; first match wins.
- **`FocusManager<T>`** (`src/state/focus.rs`, ~100 lines): ordered list with
  `next()/prev()/set_index()`, wraps around, auto-focuses first element.
- **`SplitPane`** (`src/components/split_pane.rs`): percentage-based area
  split with a 1-col divider, `min_percent`/`max_percent` clamping,
  drag-to-resize state machine (`start_drag` / `update_drag` / `end_drag`),
  keyboard resize when `divider_focused`, orientation H/V.
  `calculate_areas(area, split_percent) -> (first, divider, second)` is pure
  math and directly portable.
- **`MouseCaptureState`** (`src/utils/mouse_capture.rs`): runtime
  enable/disable of crossterm mouse capture ("copy mode").

### 3.2 `rat-salsa` rat-focus — most complete mouse hit-test

- Each widget registers `(Rect, z_index)`; `focus_at(col, row)` finds the
  topmost widget/container whose rect contains the point and moves keyboard
  focus there; `mouse_focus(col, row)` sets hover flags for all containing
  widgets. Supports nested containers (a pane containing sub-regions) and
  z-order for overlapping areas (popups over panes).
- `HandleEvent<Event, Qualifier, Return>` trait with `Regular` / `MouseOnly` /
  `Dialog` qualifiers; `Outcome::Continue/Unchanged/Changed` + `is_consumed()`.
- Requires every widget to implement `HasFocus` with a `FocusFlag` and a
  per-frame `FocusBuilder` rebuild — **too invasive to adopt wholesale**; the
  click-to-focus + z-index *concept* is what we borrow.

### 3.3 `ratatui-kit` InputRuntime — best keyboard isolation model

- Central `InputRuntime`: per-frame layer stack; `InputLayer { blocks_lower }`
  (a focused pane's layer blocks lower layers); `EventScope` (Current / Layer /
  Global); `EventPriority` (Low/Normal/High); `EventOptions { hit_test }` —
  mouse events delivered only if inside the component's previous-frame `Rect`.
- Dispatch: Global phase first, then active layers by (z-order desc, priority
  desc, registration asc); `EventResult::Consumed` stops propagation.
- **`EventResult::Consumed/Ignored` is identical to fspec's existing enum** —
  the model ports into the existing `App::handle_event` cascade without
  new concepts.
- It's a full React-style component framework (`element!`, `#[component]`,
  hooks) — adopting it would rewrite all of fspec's views. We port the
  *design* (~250-line runtime), not the framework.

### 3.4 `focusable` / `widgetui`

- `focusable`: pure focus *state* (`#[derive(Focus)]`, `focus_next/prev`), no
  event routing, no mouse. Experimental.
- `widgetui`: bevy-like widget-tree framework; different paradigm; overkill.

### 3.5 Conclusion

Do **not** adopt any framework wholesale. Build a small `MultiplexLayout` in
`views/multiplex/` that reuses patterns already present in fspec:

- fspec already caches per-region `Rect`s in `Cell`s for mouse hit-testing
  (`BoardView::last_column_content_areas`, `ChangedFilesView::last_files_rect`).
- fspec already routes events by active view with `EventResult`
  (`Navigator::handle_event`).
- fspec's views use **absolute terminal coordinates** for all cached rects, so
  rendering a view into a sub-`Rect` works with zero coordinate translation —
  exactly what `ChangedFilesView` already does.

## 4. Proposed Design

### 4.1 New types

```text
rust/fspec-tui/src/
  views/multiplex/
    mod.rs          # MultiplexLayout struct + pane kind enum + new()
    layout.rs       # calculate_areas: orientation + split percents -> Rects
    render.rs       # render_with_stores: split area, render panes + divider
    keys.rs         # keyboard routing: focus cycling, divider resize, forward
    mouse.rs        # mouse hit-test: click-to-focus, divider drag
    presets.rs      # named grid presets (default, board-agent, quad...)
    tests.rs        # unit tests
  store/mux_state.rs    # MuxState: persisted config (orientation, splits,
                        # pane list, focused pane) + serde
  app/dispatch_mux.rs   # App::dispatch arms for the new Actions
```

```rust
/// Which top-level view a mux pane hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuxPaneKind {
    Board,
    Agent,
    ChangedFiles,
    Checkpoints,
}

/// Mux grid orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuxOrientation {
    Horizontal, // left | right | ...
    Vertical,   // top / bottom / ...
}

/// Mux grid configuration (persisted to spec/mux.json or similar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuxConfig {
    pub orientation: MuxOrientation,
    /// Split percentages for panes 0..n-1; last pane takes the remainder.
    pub splits: Vec<u16>,
    pub panes: Vec<MuxPaneKind>,
    pub focused_pane: usize,
    pub enabled: bool,
}

/// Live layout state (not persisted).
pub struct MultiplexLayout {
    pub config: MuxConfig,
    /// Cached absolute pane rects from last render (mouse hit-testing).
    pub pane_rects: Vec<Rect>,
    pub divider_rect: Option<Rect>,
    pub divider_focused: bool,
    pub is_dragging: bool,
    drag_start: (u16, u16),      // (position, original split percent)
}
```

### 4.2 Layout math (port from `ratatui-interact::SplitPane::calculate_areas`)

- N panes, orientation O, split percents `s[0..n-1]` (last pane = remainder):
  - available = total − (n−1) divider cols/rows
  - pane_i = clamp(available × s[i] / 100, min_pane_width, available − Σmin)
- **`min_pane_width` default 52** (board needs ≥ 51; agent works narrower but
  52 keeps the input row usable). If the terminal can't fit all panes at the
  minimum, degrade: drop trailing panes into a "hidden" list and show a
  `+N` indicator in the divider (or simply clamp and show what fits).
- Divider: 1 col/row, painted `│` / `─`, highlighted when `divider_focused`,
  cyan while dragging (mirrors `SplitPaneStyle` defaults).

### 4.3 Rendering

```text
MultiplexLayout::render_with_stores(area, buf, board_store, agent_store):
  1. Compute pane rects via layout math (cache into pane_rects/divider_rect).
  2. For each pane: dispatch by MuxPaneKind:
       Board         -> BoardView::render_with_store(rect, buf, board_store)
       Agent         -> AgentView::render_with_store(rect, buf, agent_store)
       ChangedFiles  -> Navigator's changed_files.render(rect, buf)
       Checkpoints   -> Navigator's checkpoints.render(rect, buf)
  3. Paint the divider (on top).
  4. Paint a 1-row mux footer: "MUX [1/2] Board | Agent  ( /mux config,
     Shift+←/→ focus, m toggle, Esc exit )" — per RPC-013 each view paints
     its own footer; the mux footer replaces the single-view footer while
     active.
```

Unfocused panes render normally (they're live views, not snapshots); the
focused pane gets a highlighted border/divider accent. No dimming in MVP.

### 4.4 Event routing (the "trap")

Inside `Navigator::handle_event`, when `active_view == ViewMode::Mux`:

```text
Mouse event:
  1. Hit-test against divider_rect first:
       Down(Left)  -> start drag (record position + original split)
       Drag(Left)  -> update split percent (clamped to min/max)
       Up(Left)    -> end drag
  2. Else hit-test pane_rects:
       -> focused_pane = hit index   (click-to-focus, rat-focus pattern)
       -> forward the event to that pane's view handler
  3. Else: ignored (click in a gap)

Key event (Press only — existing central filter applies):
  1. 'm' (no modifiers)        -> toggle mux off (emit Action::MuxToggle)
  2. Shift+Left / Shift+Right  -> cycle focused_pane (wrap-around;
                                   emits Action::MuxFocusNext/Prev so the
                                   App reducer can mirror state)
  3. Tab                       -> cycle focused_pane (parity with
                                   ChangedFilesView)
  4. divider_focused:
       Left/Right (H) or Up/Down (V) -> adjust split ±2%
       Home/End                      -> min/max split
       Esc                           -> divider_focused = false
  5. Everything else -> forward to the FOCUSED pane's view handler only.
     (Unfocused panes receive NO keyboard events — this is the isolation.)
```

**Important**: forward the *original* event to the pane handler. Because all
view-internal hit-testing uses absolute coordinates and the views are rendered
into their sub-rects, no coordinate translation is needed (verified:
`BoardView` caches `last_column_content_areas` etc. as absolute rects from the
rendered `area`).

**Key-conflict handling**: while mux is active, `Shift+Left/Right` mean
*pane focus* (not session cycling). Session cycling inside the focused agent
pane moves to `Ctrl+Left/Right` (new binding, documented in the mux footer and
help dialog). `Enter` on the board pane keeps its existing meaning
(`EnterWorkUnit`), but in mux mode the App reducer should **not** flip
`active_view` to Agent — instead it should focus/attach the agent pane
(`Action::MuxEnterWorkUnit`). `Esc` on the board pane in mux mode exits mux
mode (back to Board) rather than opening the exit confirmation.

### 4.5 New `Action` variants (`components/mod.rs`)

```rust
/// Toggle mux mode on/off (slash command + 'm' key).
MuxToggle,
/// Apply a parsed /mux configuration (orientation, splits, pane list).
MuxConfigApplied(MuxConfig),
/// Cycle pane focus left/right (Shift+arrows / Tab in mux mode).
MuxFocusPrev,
MuxFocusNext,
/// Adjust the split between the focused pane and its neighbor.
MuxSplitAdjust(i16),
/// Enter a work unit from the board pane while in mux mode (focus the
/// agent pane instead of flipping the whole view).
MuxEnterWorkUnit(String),
/// Exit mux mode, returning to the single Board or Agent view.
MuxExit(Option<ViewMode>),
```

### 4.6 The `/mux` slash command

Parsed in `app/slash_parser.rs` (or a sibling `mux_parser.rs` to respect the
300-line ceiling) and dispatched in `app/dispatch_slash_commands.rs`:

```text
/mux                  -> toggle mux on/off (on: apply saved/default config)
/mux off              -> disable mux, return to current view
/mux on               -> enable mux with saved/default config
/mux h                -> horizontal orientation
/mux v                -> vertical orientation
/mux 2                -> 2 panes (default: Board | Agent)
/mux 3                -> 3 panes (Board | Agent | ChangedFiles)
/mux 4                -> 4 panes (Board | Agent | ChangedFiles | Checkpoints)
/mux board agent      -> explicit pane list (order = layout order)
/mux board agent 40   -> pane list + first split percent (40/60)
/mux 50 50            -> split percentages for the current pane count
/mux save             -> persist current config to disk
/mux default          -> reset to default preset (horizontal, Board|Agent, 50/50)
/mux help             -> show available subcommands (footer or dialog)
```

Grammar (simple, space-separated):

```text
mux := "mux" ( "on" | "off"
             | orientation
             | pane-count
             | pane-list (split-percents)?
             | "save" | "default" | "help" )
orientation  := "h" | "v" | "horizontal" | "vertical"
pane-count   := integer 2..=4
pane-list    := pane-kind { pane-kind }        # 2..=4 items
pane-kind    := "board" | "agent" | "files" | "checkpoints"
split-percents := integer 10..=90 { integer 10..=90 }   # count-1 values
```

Parse errors (unknown pane kind, out-of-range percent, too many panes for the
terminal width) surface as a one-line error in the agent scrollback / a
transient status line — never a blocking dialog — and leave the current mux
config untouched.

**Persistence**: config saved via `/mux save` (and auto-saved on exit) to
`spec/mux.json` (next to the other spec state; confirm exact location during
discovery — follow whatever convention `spec/` already uses for TUI state).
Loaded at bootstrap; missing file → default preset.

### 4.7 Navigator integration

```rust
// views/navigator.rs
pub enum ViewMode {
    Board, Agent, ProviderSettings, Blocklist, ModelSelector,
    ChangedFiles, Checkpoints,
    /// MUX-001: multiplex grid of top-level views.
    Mux,
}

pub struct Navigator {
    // ... existing fields ...
    pub mux: MultiplexLayout,
}
```

- `apply_action`: `MuxToggle`/`MuxConfigApplied` flip `active_view` to/from
  `Mux`; `MuxExit(Some(mode))` restores the previous single view (store the
  pre-mux `ViewMode` in `MuxState` so exit is deterministic).
- `handle_event`: new `ViewMode::Mux` arm delegates to
  `MultiplexLayout::handle_event` (which forwards to the child views).
- `render_with_stores`: new `ViewMode::Mux` arm delegates to
  `MultiplexLayout::render_with_stores`.
- `is_view_loading`: mux is loading iff any pane is (Checkpoints/ChangedFiles
  panes can be mid-cascade).

### 4.8 State placement

- **`MuxState` in `AgentViewStore`? No.** Mux config is app-level UI state,
  not session state. It lives on `App` (a `mux_state: MuxState` field,
  `store/mux_state.rs`), mutated only in `App::dispatch`, mirroring how
  `BoardStore`/`AgentViewStore` are owned by `App`.
- Per-pane *view* state (board selection, agent input drafts, changed-files
  selection) stays in the existing stores — mux mode does not duplicate it.
  The focused-pane input draft round-trip reuses the existing
  `AgentViewStore::set_input_draft` / `input.set_value` pattern from
  `dispatch_session_cycle.rs` when focus moves in/out of the agent pane.

### 4.9 Help + footer

- Mux footer (1 row, bottom): `MUX 2 panes [●Board|Agent]  /mux config ·
  m off · Shift+←/→ focus · drag divider`.
- Help dialog (`components/help_content.rs`): new "Mux mode" section listing
  all bindings and `/mux` subcommands.
- Board keybinding strip (`views/board/keybinding_shortcuts.rs`): add `m` hint
  when mux is available.

## 5. ACDD Plan (for the implementing agent)

### 5.1 Work unit

- Create a **story** work unit, prefix `MUX` (register prefix first),
  title: "Mux mode — multiplexed top-level views with /mux configuration".
- Epic: `tui` (or create `mux` epic if `tui` doesn't exist).
- Estimate: **8** (upper-moderate: new view mode, slash parser, persistence,
  mouse + keyboard routing, layout math). If discovery reveals the
  key-conflict surface is larger, re-estimate to 13 and split.

### 5.2 Discovery / Example Mapping

- User story: *As a developer supervising multiple agents, I want to see the
  board and an agent conversation side-by-side in a configurable grid, so
  that I can monitor progress without switching views.*
- Rules (blue cards) to elicit/confirm:
  - R1: `/mux` toggles mux on/off; `/mux off` returns to the pre-mux view.
  - R2: keyboard input goes only to the focused pane; clicking a pane
    focuses it.
  - R3: Shift+Left/Right cycle pane focus while mux is active (session
    cycling moves to Ctrl+Left/Right).
  - R4: the divider is drag-resizable (mouse) and keyboard-resizable when
    focused (Tab onto it or click it).
  - R5: no pane may be narrower than 52 columns (board minimum); splits are
    clamped, never allowed to produce a sub-minimum pane.
  - R6: config persists across restarts when saved; missing file → default
    preset (horizontal, Board|Agent, 50/50).
  - R7: parse errors in `/mux` leave the current config untouched and show a
    one-line error.
  - R8: `Enter` on a board work unit in mux mode focuses the agent pane for
    that unit (does NOT flip the whole screen to Agent).
  - R9: mux mode coexists with the Compositor: dialogs (help, exit
    confirmation, create-session) still overlay the full screen and capture
    input above mux.
  - R10: existing single-view behavior is byte-for-byte unchanged when mux is
    off (all existing tests stay green).
- Examples (green cards): at least one per rule above, e.g. "User types
  `/mux 3` in the agent input; the screen becomes Board | Agent | ChangedFiles
  with the agent pane focused; typing `hello` lands in the agent pane only."
- Questions (red cards): ask the human about (a) default preset, (b) whether
  `m` as the toggle key is acceptable (vs a different key), (c) persistence
  file location, (d) whether the divider should be focusable via Tab in MVP.

### 5.3 Feature file

`spec/features/rust-mux-mode.feature` (capability name, NOT the work-unit id),
tags: `@MUX-001`, `@wip`, plus component/phase tags per `spec/TAGS.md`
(register new tags via `fspec register-tag` if needed).

Scenarios (minimum, one per rule):
1. `/mux` toggles mux mode on with the default preset
2. `/mux off` returns to the pre-mux view
3. `/mux h|v` sets orientation
4. `/mux <n>` sets pane count with default pane kinds
5. `/mux board agent 40` sets explicit pane list + split
6. `/mux` with an invalid pane kind leaves config unchanged and shows an error
7. keyboard input routes only to the focused pane
8. clicking a pane focuses it and routes the click to that pane
9. Shift+Left/Right cycle pane focus (wrap-around)
10. divider drag resizes the split (clamped to min pane width)
11. keyboard divider resize when divider focused
12. split clamping never produces a pane < 52 cols
13. config persists to disk and reloads at bootstrap
14. `Enter` on board work unit in mux mode focuses the agent pane
15. dialogs overlay mux mode and capture input
16. existing single-view behavior unchanged when mux is off

### 5.4 Tests (before implementation)

- Unit tests for `MuxConfig` parsing (`mux_parser.rs`): proptest for the
  grammar (random valid/invalid inputs), insta snapshots for error messages.
- Unit tests for layout math: pure functions, boundary cases (tiny terminal,
  all-min splits, 2/3/4 panes, H/V).
- Unit tests for routing: synthetic `Event`s through
  `MultiplexLayout::handle_event` asserting which pane handler received them
  (mirror the existing `navigator.rs` test style with `TestBackend`).
- Integration tests: `App`-level — send `/mux` via the action bus, assert
  `active_view == Mux`, render into a `TestBackend`, assert both panes'
  content appears in the expected column ranges.
- All tests use `rust/test-helpers/` for temp dirs (persistence tests),
  `serial_test` where global state is mutated, and follow the
  `// @step Given/When/Then` comment convention for Gherkin mapping.

### 5.5 Implementation order

1. `MuxConfig` + parser + tests (pure, no UI).
2. `MuxState` + persistence + tests.
3. `MultiplexLayout` layout math + tests (pure).
4. `MultiplexLayout::render_with_stores` + render tests (TestBackend).
5. `ViewMode::Mux` wiring in `Navigator` + `App::dispatch` arms.
6. Keyboard routing + tests.
7. Mouse routing (click-to-focus, divider drag) + tests.
8. `/mux` slash command integration + tests.
9. Help/footer/keybinding-strip updates.
10. `fspec validate`, `fspec validate-tags`, link coverage, run full
    `cargo test -p fspec-tui`, clippy, fmt.

### 5.6 Definition of done

- All scenarios in `rust-mux-mode.feature` have passing tests with `@step`
  comments and linked coverage.
- `cargo clippy -p fspec-tui` clean (workspace denies apply).
- No existing test regressed (R10).
- Feature file formatted + validated; tags registered; work unit moved to
  `done`; `@wip` removed, `@done` added.

## 6. Out of Scope (explicit)

- Multiple simultaneous *live* agent panes (would require N `AgentView`
  instances + widening the chunk subscriber filter to `HashSet<SessionId>`).
  A second work unit can layer this on: render additional agent panes as
  read-only previews (scrollback only) using per-session `SessionContext`
  scrollback, which is already stored per session.
- Nested mux (mux within a mux pane).
- Mouse wheel routing per pane (nice-to-have; the existing per-pane wheel
  handlers work if the event is forwarded to the hit pane — verify in
  implementation, add a scenario if it works for free).
- Per-pane independent footers (MVP uses the single mux footer row).

## 7. File-Level Change Map

| File | Change |
|------|--------|
| `rust/fspec-tui/src/views/multiplex/mod.rs` | NEW: `MultiplexLayout`, `MuxPaneKind`, `MuxOrientation` |
| `rust/fspec-tui/src/views/multiplex/layout.rs` | NEW: pure split math + clamping |
| `rust/fspec-tui/src/views/multiplex/render.rs` | NEW: pane dispatch + divider + footer paint |
| `rust/fspec-tui/src/views/multiplex/keys.rs` | NEW: keyboard routing |
| `rust/fspec-tui/src/views/multiplex/mouse.rs` | NEW: hit-test, click-to-focus, divider drag |
| `rust/fspec-tui/src/views/multiplex/presets.rs` | NEW: default/named presets |
| `rust/fspec-tui/src/views/multiplex/tests.rs` | NEW: unit tests |
| `rust/fspec-tui/src/views/mod.rs` | export `multiplex` module |
| `rust/fspec-tui/src/views/navigator.rs` | `ViewMode::Mux`, `mux` field, 3 match arms |
| `rust/fspec-tui/src/store/mux_state.rs` | NEW: `MuxState` + serde persistence |
| `rust/fspec-tui/src/store/mod.rs` | export `mux_state` |
| `rust/fspec-tui/src/components/mod.rs` | new `Action` variants |
| `rust/fspec-tui/src/app/mux_parser.rs` | NEW: `/mux` grammar parser |
| `rust/fspec-tui/src/app/slash_parser.rs` | route `mux` to `mux_parser` |
| `rust/fspec-tui/src/app/dispatch_mux.rs` | NEW: `App::dispatch` arms |
| `rust/fspec-tui/src/app/dispatch.rs` | delegate mux arms |
| `rust/fspec-tui/src/app/state.rs` | `mux_state` field + accessors |
| `rust/fspec-tui/src/app/bootstrap.rs` | load mux config at bootstrap |
| `rust/fspec-tui/src/views/agent/dispatch.rs` | Ctrl+Left/Right session cycling (mux-aware) |
| `rust/fspec-tui/src/components/help_content.rs` | mux help section |
| `rust/fspec-tui/src/views/board/keybinding_shortcuts.rs` | `m` hint |
| `spec/features/rust-mux-mode.feature` | NEW: acceptance criteria |
