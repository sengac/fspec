# RPC-381 — Port AgentView Tab turn-selection (SELECT) mode to Rust

## 1. Problem Statement

The TypeScript reference `AgentView` (`src/tui/components/AgentView.tsx`, ~5,685 lines)
supports a **turn-selection mode** toggled by the **Tab** key (`isTurnSelectMode`, feature
tag TUI-042). The Rust port (`codelet/fspec-tui/src/views/agent/`) is missing this
feature entirely:

- There is **no `Tab` handler** in the agent dispatch (`views/agent/dispatch.rs`). Tab is
  only handled inside popups.
- `ScrollbackList` (`views/agent/scrollback.rs`) is **scroll-only** — it tracks a
  `ScrollState { offset, stick_to_bottom }` and has no concept of a *selected turn*,
  no `SelectionMode`, no item navigation.
- The header **`[SELECT]` badge already exists** in `header_build.rs:87-92` but is
  **hardwired off** at `chrome_paint.rs:60` (`is_select_mode: false`).
- There is no arrow-bar highlight (`▼`/`▲`) framing the selected turn.

This card ports the **core SELECT mode**. The turn-content modal that opens on `Enter`
is split into the dependent card **RPC-382**.

## 2. TypeScript Reference Behaviour (what we are porting)

### 2.1 State (`AgentView.tsx:854-862`)
- `isTurnSelectMode` (default `false`) — master toggle.
- `virtualListSelectionRef = { selectedIndex: 0 }` — lets the parent read the selected
  index from the `VirtualList`.

### 2.2 Tab toggles the mode (`AgentView.tsx:4836-4847`)
```ts
if (key.tab) {
  const newMode = !isTurnSelectMode;
  setIsTurnSelectMode(newMode);
  if (!newMode) { setShowTurnModal(false); setModalMessageIndex(null); }
  return true;
}
```
Tab flips the boolean. It does **not** manually set a selection index — it relies on the
`VirtualList` auto-selecting the **last** turn when it transitions into item mode.

### 2.3 VirtualList re-wiring (`AgentView.tsx:5460-5476`)
```ts
scrollToEnd={true}
selectionMode={isTurnSelectMode ? 'item' : 'scroll'}
groupBy={isTurnSelectMode ? line => line.messageIndex : undefined}
groupPaddingBefore={isTurnSelectMode ? 1 : 0}
selectionRef={virtualListSelectionRef}
isFocused={... && !showTurnModal}
```

### 2.4 Auto-select last turn (`VirtualList.tsx:343-350`)
On a `scroll → item` transition with `scrollToEnd`, `selectedIndex = totalItemCount - 1`.

### 2.5 Turn navigation (`VirtualList.tsx:436-490, 599-626`)
`navigateToGroup('up'|'down')` jumps turn-to-turn (whole message group), not line-to-line.
↑/↓ → group nav; PageUp/Down → by `visibleHeight`; Home/End → first/last group.

### 2.6 Arrow-bar highlight (`turnSelection.ts` + `AgentView.tsx:5325-5343`)
- `getSelectionSeparatorType(line, lineIndex, allLines, selectedIndex, isSelectMode)`
  returns `'top'` (▼ bar above the selected turn), `'bottom'` (▲ bar below), or `null`.
- `generateArrowBar(width, direction, spacing=4)` builds `"▼   ▼   ▼"` / `"▲   ▲   ▲"`.
- Rendered on a **gray background, white foreground** (`backgroundColor="gray"`).

### 2.7 Header badge (`AgentView.tsx:5283`)
`SessionHeader` receives `isSelectMode={isTurnSelectMode}` → shows a SELECT badge.

### 2.8 Suppress Enter-submit (`AgentView.tsx:5516`)
`suppressEnter` includes `isTurnSelectMode` so Enter does not submit input while selecting.

### 2.9 Esc exits select mode (`AgentView.tsx:4792-4834`)
Esc priority cascade. The relevant level for THIS card:
- Priority 4: if `isTurnSelectMode` → `setIsTurnSelectMode(false)` and consume.
(The "close turn modal" level above it is RPC-382.)

## 3. Rust Architecture — Current State

| Concern | Rust location | Status |
|---|---|---|
| Key dispatch | `views/agent/dispatch.rs` `handle_event` | No Tab handler |
| Scrollback widget | `views/agent/scrollback.rs` `ScrollbackList` | Scroll-only |
| Chunk model | `views/agent/rendered_chunk.rs` `RenderedChunk { seq, lines, source }` | Has `seq` (usable group id) |
| Row painting | `views/agent/scrollback_paint.rs` `paint_chunk_rows` | No selection awareness |
| Header badge | `views/agent/header_build.rs:87` (`is_select_mode`) | Exists; wired to `false` |
| Header wiring | `views/agent/chrome_paint.rs:60` | Hardwired `is_select_mode: false` |
| AgentView struct | `views/agent.rs:85` | Presentation state; add toggle here |
| Per-session state | `store/agent_view/session_context.rs` `SessionContext` | Owns `scrollback` |
| Actions | `components/mod.rs` `enum Action` | Add new variants |

### Key simplification vs TS
In the Rust port **each `RenderedChunk` already corresponds to one conversation message**
(UserInput / AssistantText / Thinking / ToolCall chunks). There are **no interleaved
separator lines** inside the scrollback the way the TS `VirtualList` has. Therefore a
"turn" = **one `RenderedChunk`, keyed by `seq`**. This is materially simpler than the TS
`groupBy(messageIndex)` logic — no group-walking across separator lines is required.
Navigation = move the selected chunk index by ±1; highlight = frame the selected chunk's
visible rows with arrow bars.

## 4. Recommended Design

### 4.1 Where state lives
- **`AgentView.turn_select_mode: bool`** (in `views/agent.rs`) — mirrors the TS
  component-level `isTurnSelectMode` (persists across Shift+session cycling, matching TS).
- **Selection cursor on `ScrollbackList`** — add a `SelectionMode { Scroll, Item }` and a
  `selected: Option<usize>` (index into `chunks`). Storing it on the scrollback means each
  `SessionContext` preserves its own selection, and the renderer has direct access.

> Alternative considered: store selection in `SessionContext`. Rejected — the renderer and
> navigation all operate on `ScrollbackList`, so co-locating selection there is cohesive
> (Single Responsibility: the list owns both its scroll AND its selection).

### 4.2 `ScrollbackList` additions (`scrollback.rs`)
```rust
pub enum SelectionMode { Scroll, Item }

// new fields:
//   selection_mode: SelectionMode (default Scroll)
//   selected: Option<usize>       (index into chunks)

pub fn enter_item_mode(&mut self);   // set Item + select_last_turn()
pub fn exit_item_mode(&mut self);    // set Scroll + selected = None
pub fn select_last_turn(&mut self);  // selected = chunks.len().checked_sub(1)
pub fn navigate_turn(&mut self, dir: TurnDir); // Up/Down, clamp at ends
pub fn selected_index(&self) -> Option<usize>;
pub fn selected_seq(&self) -> Option<u64>;
```
- `navigate_turn` must keep the selected chunk visible: after moving, adjust
  `scroll_state.offset` so the selected chunk's row span (plus the two arrow-bar rows)
  is within the viewport. Port of `getVisibleRange` + scroll-to-keep-visible
  (`VirtualList.tsx:368-420`). Because a turn is one chunk, `getVisibleRange` reduces to
  "first visual row of chunk N .. last visual row of chunk N".
- **Selection preservation across streaming**: persist the selected **`seq`** (stable id),
  not the index. When `chunks` mutate (push/insert), re-resolve the index from the seq so
  the selection sticks to the same turn (port of `VirtualList.tsx:258-274`).

### 4.3 Arrow-bar rendering (`scrollback_paint.rs`)
Port two pure helpers:
```rust
pub(super) fn generate_arrow_bar(width: usize, dir: ArrowDir /*Top|Bottom*/, spacing: usize) -> String
// Top => '▼', Bottom => '▲'; one arrow every `spacing` (4) cols, spaces elsewhere.
```
When `selection_mode == Item` and a chunk is selected, `paint_chunk_rows` (or a wrapper)
must paint, on a **gray background / white foreground** `Style`:
- a `▼   ▼   ▼` bar row immediately **above** the selected chunk's first row, and
- a `▲   ▲   ▲` bar row immediately **below** the selected chunk's last row.

These two extra rows count toward layout/viewport math — `select_last_turn` /
`navigate_turn` visibility math must account for the +2 rows so the bars never clip.

### 4.4 Dispatch wiring (`dispatch.rs`)
1. Add a Tab handler in `handle_event` (after popups/mode-views are checked, since those
   consume Tab themselves):
   ```rust
   if key.code == KeyCode::Tab && key.modifiers.is_empty() {
       self.emit(Action::ToggleTurnSelectMode);
       return EventResult::consumed();
   }
   ```
2. When `turn_select_mode` is true, route ↑/↓/PageUp/PageDown/Home/End to **turn
   navigation** actions instead of the scrollback line/page-scroll actions.
3. Suppress Enter-submit while in select mode (Enter must NOT submit). RPC-382 will make
   Enter open the modal; for THIS card, Enter is simply suppressed (no-op) in select mode.
4. Extend the Esc handling: when `turn_select_mode` is true, Esc exits select mode and
   consumes the event (do not fall through to `AgentEscPressed`).

### 4.5 Actions / reducer (`components/mod.rs` + App dispatch)
Add:
- `Action::ToggleTurnSelectMode` — flips `AgentView.turn_select_mode`; on enable, calls the
  current session's `scrollback.enter_item_mode()`; on disable, `exit_item_mode()`.
- `Action::TurnNavUp` / `Action::TurnNavDown` — call `scrollback.navigate_turn(..)`.

(Reducing in the App task mirrors how existing `Scrollback*` actions are handled.)

### 4.6 Header badge (`chrome_paint.rs`)
Replace the hardwired `is_select_mode: false` (line 60) with the real flag. Since
`chrome_paint::paint_header_and_role` does not currently receive `AgentView`, thread the
`turn_select_mode` bool through (e.g. add a parameter, or read it from a store flag set by
the reducer). Prefer adding a parameter to keep the store free of pure-presentation state,
OR add a minimal `turn_select_mode` flag to `AgentViewStore` if the call site cannot see
the `AgentView`. Document the chosen approach in the work-unit architecture notes.

## 5. Acceptance-Test Strategy (Rust / `cargo test`)

Follow the existing `views/agent/*tests*` + `tests/view_agent_unit_*.rs` conventions.

- **Unit (scrollback)**: `enter_item_mode` selects the last chunk; `navigate_turn(Up)`
  moves to the previous chunk and clamps at index 0; `navigate_turn(Down)` clamps at the
  last chunk; selection follows the same `seq` after a `push`.
- **Render**: with item mode + a selected chunk, the buffer contains a `▼` arrow-bar row
  above and a `▲` arrow-bar row below the selected chunk, both with gray-bg style.
- **`generate_arrow_bar`**: arrow glyph + spacing pattern parity (`▼   ▼`).
- **Dispatch**: Tab emits `Action::ToggleTurnSelectMode`; in select mode ↑/↓ emit
  `TurnNavUp/Down`; Enter is suppressed (no `InputSubmitted`); Esc emits the
  exit-select-mode path, not `AgentEscPressed`.
- **Header**: `build_left_line(..., is_select_mode = true, ...)` includes the ` [SELECT]`
  span (already covered by header tests; add the end-to-end wiring assertion).

## 6. Out of Scope (handled elsewhere)
- **Enter → turn content modal**, full-content source on chunks, and the "Esc closes modal
  first" cascade level → **RPC-382** (depends on this card).
- "Discuss Selected" prefill (`getFirstContentOfTurn` / `generateDiscussSelectedPrefill`)
  — exported TS utilities NOT wired into AgentView's Enter path; subordinate-pane feature,
  not part of this port.

## 7. Reference File Index
| Purpose | TS | Rust |
|---|---|---|
| Tab toggle | `AgentView.tsx:4836-4847` | `views/agent/dispatch.rs` (add) |
| Mode state | `AgentView.tsx:854` | `views/agent.rs:85` (add field) |
| VirtualList item mode | `VirtualList.tsx:150,343-350,599-626` | `views/agent/scrollback.rs` (add) |
| Arrow bars | `turnSelection.ts:23-74` | `views/agent/scrollback_paint.rs` (add) |
| Header badge | `AgentView.tsx:5283` / SessionHeader | `header_build.rs:87` (exists), `chrome_paint.rs:60` (wire) |
| Esc cascade | `AgentView.tsx:4792-4834` | `views/agent/dispatch.rs:182` (extend) |
