# RPC-012 — Implementation Plan

**Title:** Base refactor: dedicated `BoardStore` + `AgentViewStore` + `RootView` navigator (Board/Agent)
**Parent:** RPC-002 · **Epic:** rust-frontend · **Status:** specifying
**Author note:** This document is the detailed brief for the slice. Read it top-to-bottom before answering the open questions at the bottom — those answers feed `fspec generate-scenarios RPC-012`.

---

## 1. Why this card exists

The TypeScript fspec TUI (under `src/tui/`) is the contract our Rust port (`codelet/fspec-tui/`) must eventually match. The current Rust TUI — built across RPC-005..011 — landed a working but **flat** UI: a left-pane work-unit list and a right-pane single-session agent REPL, both inside one fixed `RootView` with all state living in component fields.

The TypeScript reference is structurally different in three ways that matter:

| TS surface                               | Rust today                          | Gap                                                |
|------------------------------------------|-------------------------------------|----------------------------------------------------|
| `UnifiedBoardLayout.tsx` — 7-col Kanban + details strip + `[/]` reorder + session indicators | `WorkUnitsListView` — flat j/k list | Full Kanban surface                                |
| `AgentView.tsx` — full-screen, multi-session, slash commands, model picker, history | `AgentReplView` — single-session, single-line input | Eventual full port                                 |
| `fspecStore.ts` (Zustand)                | All state in component fields       | **No board store**                                 |
| `sessionStore.ts` (Zustand)              | `AgentReplView.active_session` field only | **No agent view store**                            |
| `BoardView → AgentView` via Enter or Shift+Right, with `navigationTargetSessionId` handoff | Tab cycles focus inside one screen  | **No top-level navigator**                         |

This card is **the base refactor only** — it lays down the state-ownership + navigation skeleton so each downstream slice (full Kanban render, multi-session AgentView, slash commands, model picker, etc.) can land cleanly without having to invent its own pattern.

The full epic review that motivated this card is at:
- [`spec/attachments/RPC-002/review-findings-2026-05-12.md`](../RPC-002/review-findings-2026-05-12.md)

Critical drifts from RPC-009 (left-pane width `Length(50)` vs `Length(32)`, render format `{id} [{status}] {title}` vs `{id} {status}`, `Action` enum lost `PartialEq`/`Eq`) are **absorbed** by this card because the offending views are being removed/replaced.

---

## 2. Scope summary

### What this card delivers (BASE ONLY — not the full UI)

1. **New module `codelet/fspec-tui/src/store/`** containing:
   - `BoardStore` — single source of truth for work-units list grouped by status, focused column, per-column selection, session attachments map, last-changed indicator.
   - `AgentViewStore` — single source of truth for current session, navigation target, current work-unit id/status, create-session dialog flag, auto-create flag.
2. **`RootView` refactor → top-level navigator** with `active_view: ViewMode { Board, Agent }`, rendering exactly ONE child view per frame plus the 1-row footer. The previous fixed two-pane (list + REPL) layout is removed.
3. **New `BoardView`** — first landing view, reads from `BoardStore`, emits new actions. Minimal placeholder Kanban skeleton acceptable in this slice (the rich `UnifiedBoardLayout` port is downstream).
4. **`AgentView` wrapper** (renamed/refactored from `AgentReplView`) — shown only when `active_view == Agent`, reads `current_session` from `AgentViewStore` (not from its own field). Single-session, single-line input preserved.
5. **New `Action` variants** — `EnterWorkUnit(String)`, `OpenAgentView(Option<SessionId>)`, `BackToBoard`, `NavigationTargetSet(Option<SessionId>)`. The existing `FocusNext` is repurposed (or replaced) at the navigator level.
6. **Wiring in `App::dispatch`** that routes the new actions and mutates the stores synchronously on the App task.
7. **Lazy session creation** — `App::bootstrap` no longer pre-creates a session; `create_session(None)` is deferred until the user first enters `AgentView`.
8. **Source-shape regression tests** widened to forbid `Mutex`/`RwLock`/runtime constructors/napi imports inside the new `store/` module.
9. **All files under 300 LoC** — the new modules ship under 300 LoC and the existing drifts (`app.rs` 604, `views/agent_repl.rs` 519, `views/work_units_list.rs` 423, `views/root.rs` 328) are resolved as part of this refactor.

### Explicitly out of scope (each is a downstream slice)

- The full 7-column `UnifiedBoardLayout` render (column headers, viewport math, per-column scroll arrows, last-changed `⏩` animation, work-unit details strip, `[ ]` priority reorder UX, mouse SGR scroll, Home/End, PageUp/PageDown, etc.). This slice only ships a **placeholder Kanban skeleton** that proves the `BoardStore` wiring.
- The full `AgentView` port (multi-session, slash commands, file search, model picker, thinking-level dialog, history navigation, isolation, blocklist, role banner, etc.).
- Per-session `BoardStore` mutations (attach/detach session, last-changed timestamp tracking via `stateHistory`) — those land when their UI counterparts ship.
- Checkpoint counts, file status, epics, stashes — none are needed for the placeholder skeleton; add when the rich rendering arrives.
- Persistent store between runs (the TS Zustand state is also in-memory only — nothing to persist).

---

## 3. The TypeScript target (reference)

### `src/tui/components/UnifiedBoardLayout.tsx`
7-column Kanban (`backlog → specifying → testing → implementing → validating → done → blocked`) with per-column scroll, a work-unit details strip, `[ / ]` priority reorder, session-attached indicator, last-changed indicator. Sources state via `useFspecStore` (Zustand).

### `src/tui/components/AgentView.tsx`
Full-screen agent interactions; entered via `Enter` on a work unit in `BoardView` **OR** `Shift+Right` (navigate to first attached session **OR** open create-session dialog). Sources state via `useSessionStore + useFspecStore + useModelStore` (Zustand).

### `src/tui/store/fspecStore.ts`
Work units, epics, checkpoint counts, file status, **session attachments map** (`workUnitId → sessionId`).

### `src/tui/store/sessionStore.ts`
`currentSessionId`, `navigationTargetSessionId`, `currentWorkUnitId`, `currentWorkUnitStatus`, isolation state, navigation handoff helpers (`navigateToNewSession`, `setNavigationTarget`).

Re-read these four files before writing any Rust — every field below has a TS analogue.

---

## 4. Architecture invariants this story MUST preserve

- **RPC-005 Q9** — host-supplied tokio runtime `Handle` (`tokio::spawn` only; no `Builder` / `Runtime::new`).
- **RPC-008** — `FspecBackend` trait surface; every store mutation that requires backend data still goes through `Arc<dyn FspecBackend>`.
- **Loopback-only bind** — `codelet-napi` is NOT a `[dependencies]` or `[dev-dependencies]` of `codelet-fspec-tui`.
- **File size ≤ 300 LoC per file** — strictly enforced for the new modules; existing drifts must be resolved.
- **RPC-009 single-task tenere pattern** — stores are mutated ONLY on the App task; subscriber tasks emit `Action`s, never mutate stores directly. No `Mutex` / `RwLock` / `AtomicXyz` on stores.
- **RPC-009 critical drifts absorbed** — `BoardView`'s render format and width are restated for the new Kanban skeleton; `Action` enum's `PartialEq` / `Eq` derives are reconsidered (`StreamChunk`'s 23 variants block automatic `PartialEq` — confirm or use a derive-less manual impl per RPC-009).

---

## 5. Proposed file layout

### NEW

```
codelet/fspec-tui/src/store/mod.rs
codelet/fspec-tui/src/store/board.rs
codelet/fspec-tui/src/store/agent_view.rs
codelet/fspec-tui/src/views/navigator.rs       # renamed from root.rs
codelet/fspec-tui/src/views/board.rs           # replaces work_units_list.rs
codelet/fspec-tui/src/views/agent.rs           # replaces agent_repl.rs (slimmed)
```

### REMOVED / REPLACED

```
codelet/fspec-tui/src/views/root.rs            → renamed to navigator.rs
codelet/fspec-tui/src/views/work_units_list.rs → replaced by views/board.rs
codelet/fspec-tui/src/views/agent_repl.rs      → replaced by views/agent.rs
```

### UNCHANGED

```
codelet/fspec-tui/src/views/footer.rs
codelet/fspec-tui/src/views/common.rs          (split if over 300 LoC)
codelet/fspec-tui/src/components/                (HelpDialog, DisconnectDialog, etc.)
codelet/fspec-tui/src/transport/
codelet/fspec-tui/src/app.rs                   (mutates stores; must end ≤ 300 LoC)
```


---

## 6. Store designs

### 6.1 `BoardStore` (`store/board.rs`)

Owns the work-units list, grouped into the seven column states, plus selection and session-attachment indices. Plain owned struct — no `Mutex` / `RwLock` / atomics. Mutated only on the App task.

```rust
// store/board.rs — TARGET SHAPE (illustrative; final names land in code)

use std::collections::HashMap;
use codelet_rpc::shared::{SessionId, WorkUnitInfo};

pub const COLUMN_ORDER: [&str; 7] = [
    "backlog", "specifying", "testing",
    "implementing", "validating", "done", "blocked",
];

#[derive(Debug, Default)]
pub struct BoardStore {
    work_units: Vec<WorkUnitInfo>,
    by_column: HashMap<String, Vec<usize>>,        // column → indices into work_units
    focused_column: usize,                          // 0..7
    selected_index_per_column: HashMap<String, usize>,
    session_attachments: HashMap<String, SessionId>, // work_unit_id → session_id
    last_changed_id: Option<String>,
}

impl BoardStore {
    pub fn replace_work_units(&mut self, units: Vec<WorkUnitInfo>) { /* regroup, clamp selection */ }
    pub fn column_units(&self, column: &str) -> Vec<&WorkUnitInfo> { /* … */ }
    pub fn focused_column(&self) -> &str { COLUMN_ORDER[self.focused_column] }
    pub fn focus_next_column(&mut self) { /* … */ }
    pub fn focus_prev_column(&mut self) { /* … */ }
    pub fn selected_index_for(&self, column: &str) -> usize { /* default 0 */ }
    pub fn selected_work_unit(&self) -> Option<&WorkUnitInfo> { /* … */ }
    pub fn move_selection_down(&mut self) { /* clamp */ }
    pub fn move_selection_up(&mut self) { /* clamp */ }
    pub fn attach_session(&mut self, work_unit_id: &str, session: SessionId) { /* … */ }
    pub fn session_for(&self, work_unit_id: &str) -> Option<&SessionId> { /* … */ }
    pub fn mark_last_changed(&mut self, id: &str) { /* … */ }
    pub fn last_changed(&self) -> Option<&str> { /* … */ }
}
```

**Tests (inline `#[cfg(test)]`):**
- `replace_work_units` groups correctly across all 7 columns
- `focus_next_column` wraps at index 6
- Selection clamps when a column shrinks via `replace_work_units`
- `attach_session` / `session_for` round-trip

### 6.2 `AgentViewStore` (`store/agent_view.rs`)

Owns the agent-view-specific session navigation state. Replaces `AgentReplView.active_session`. Plain owned struct.

```rust
// store/agent_view.rs — TARGET SHAPE

use codelet_rpc::shared::SessionId;

#[derive(Debug, Default)]
pub struct AgentViewStore {
    current_session: Option<SessionId>,
    navigation_target_session: Option<SessionId>,
    current_work_unit_id: Option<String>,
    current_work_unit_status: Option<String>,
    show_create_session_dialog: bool,
    should_auto_create_session: bool,
}

impl AgentViewStore {
    pub fn current_session(&self) -> Option<&SessionId> { /* … */ }
    pub fn set_current_session(&mut self, session: Option<SessionId>) { /* … */ }
    pub fn set_navigation_target(&mut self, target: Option<SessionId>) { /* … */ }
    pub fn take_navigation_target(&mut self) -> Option<SessionId> { /* consumed on apply */ }
    pub fn set_current_work_unit(&mut self, id: Option<String>, status: Option<String>) { /* … */ }
    pub fn show_create_session_dialog(&self) -> bool { /* … */ }
    pub fn request_create_session_dialog(&mut self) { /* sets both flags */ }
    pub fn clear_create_session_dialog(&mut self) { /* … */ }
}
```

**Tests (inline `#[cfg(test)]`):**
- `take_navigation_target` returns `Some` once then `None`
- `request_create_session_dialog` sets `show_create_session_dialog` AND `should_auto_create_session`
- `set_current_work_unit` accepts `(None, None)` and clears both fields

### 6.3 Ownership

The stores live on **`App`** (alongside the action bus + Compositor) — **not** inside `RootView`/`Navigator`. The view layer reads from immutable references passed in via the existing `Component` trait's render path (see Question Q3 — final ownership shape decided in specifying).

---

## 7. New `Action` variants

Added to the existing enum in `codelet/fspec-tui/src/components/mod.rs`:

```rust
pub enum Action {
    // … existing …
    EnterWorkUnit(String),
    OpenAgentView(Option<SessionId>),
    BackToBoard,
    NavigationTargetSet(Option<SessionId>),
    // ReorderUp / ReorderDown stay as placeholders — backend wiring deferred
}
```

`StreamChunk`'s 23-variant inner shape forces us to keep the existing **non-derived** equality story (see RPC-009 finding); document the chosen approach in the test file.

---

## 8. `RootView` → `Navigator` refactor

```
+-------- Navigator (full screen) ----------+
| active_view: ViewMode { Board, Agent }    |
|                                           |
|   if Board: render BoardView              |
|   if Agent: render AgentView              |
|                                           |
|   1-row footer (status / hints)           |
+-------------------------------------------+
```

- `App::bootstrap` seeds `BoardStore` via `backend.list_work_units()`.
- `active_view` starts at `Board`.
- ESC inside `AgentView` (no modal layer topmost) emits `Action::BackToBoard`.
- `Enter` inside `BoardView` on a work unit emits `Action::EnterWorkUnit(id)`.
- `Shift+Right` inside `BoardView` emits `Action::OpenAgentView(target)` where `target` comes from `BoardStore.session_for(id)`.

The compositor + `tui-popup` `HelpDialog` + `DisconnectDialog` code path is **untouched**.

---

## 9. `App::dispatch` wiring

| Action                                | Effect                                                                                                                                              |
|---------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `WorkUnitsLoaded(units)`              | `board.replace_work_units(units)`                                                                                                                   |
| `EnterWorkUnit(id)`                   | `agent_view.set_current_work_unit(Some(id), status_from_board)`; `navigator.active_view = Agent`; if `agent_view.current_session.is_none()` → spawn `backend.create_session(None)` on tokio (lazy) |
| `OpenAgentView(Some(sid))`            | `agent_view.set_navigation_target(Some(sid))`; `navigator.active_view = Agent`                                                                      |
| `OpenAgentView(None)`                 | `agent_view.request_create_session_dialog()`; `navigator.active_view = Agent` (dialog body is placeholder this slice)                               |
| `BackToBoard`                         | `navigator.active_view = Board` (board selection preserved)                                                                                         |
| `SessionCreated(sid)`                 | `agent_view.set_current_session(Some(sid))`; **see Q5** for whether App also calls `board.attach_session(current_work_unit_id, sid)`                |
| `NavigationTargetSet(target)`         | `agent_view.set_navigation_target(target)`                                                                                                          |

Subscriber tasks (`work_units_rx`, `chunks_rx`, `logs_rx`) remain spawned via `tokio::spawn` on the host `Handle` per RPC-005 Q9. The `chunks_rx` subscriber filters via the existing `watch::channel<Option<SessionId>>`; the App publishes onto that channel whenever `agent_view.current_session` changes.

---

## 10. Lazy session creation

The TS `sessionStore.navigateToNewSession` / `shouldAutoCreateSession` pattern — no session exists until the user first enters `AgentView`. The first `Action::EnterWorkUnit` (or `OpenAgentView(None)`) triggers `backend.create_session(None)` via `tokio::spawn`. On completion, `Action::SessionCreated(sid)` is enqueued, which sets `agent_view.current_session`.

Test shape (sync await vs polled tick) is **Q4**.

---

## 11. Source-shape regression tests

`tests/source_shape_stores_rpc012.rs` scans `codelet/fspec-tui/src/store/` and asserts:

- **No** `std::sync::Mutex`, `tokio::sync::Mutex`, `std::sync::RwLock`, `tokio::sync::RwLock`, `parking_lot::*`, or `Atomic*` types in public store signatures.
- **No** `tokio::runtime::Builder` or `tokio::runtime::Runtime::new` calls.
- **No** direct imports of `codelet_napi`, `codelet_core`, `tarpc`, `tokio_tungstenite` in any `store/*.rs` file.

Implementation: read each `.rs` file as a string and run forbidden-substring + simple AST-free regex assertions, mirroring the existing `source_shape_*` tests landed in RPC-005 and RPC-008.

---

## 12. Integration tests

`tests/board_agent_navigation_rpc012.rs` against `MockBackend` (the existing in-tree mock used since RPC-007):

1. **bootstrap → board seeded:** `MockBackend::list_work_units` returns 3 stub units; assert `board.column_units("backlog").len() == 1`, etc.
2. **Enter handoff:** focus `AUTH-002` in `implementing`, send `Action::EnterWorkUnit("AUTH-002")`, assert `agent_view.current_work_unit_id == Some("AUTH-002")` AND `navigator.active_view == Agent`.
3. **Shift+Right with attached session:** seed `board.attach_session("AUTH-001", SessionId("s-1"))`, send `Action::OpenAgentView(Some(SessionId("s-1")))`, assert `agent_view.navigation_target_session == Some(SessionId("s-1"))`.
4. **Shift+Right without attached session:** send `Action::OpenAgentView(None)`, assert `agent_view.show_create_session_dialog == true`.
5. **ESC back:** while in `Agent`, send `Action::BackToBoard`, assert `navigator.active_view == Board` and `board.focused_column` is unchanged.
6. **Lazy session creation:** after `Action::EnterWorkUnit("AUTH-002")` on a fresh store, assert `MockBackend.create_session` was called exactly once with `None` (test shape per Q4).
7. **Cross-transport parity:** repeat the scripted sequence against `EmbeddedFspecBackend` and `WebSocketFspecBackend`; assert final store snapshots are semantically identical.

---

## 13. File-size compliance

After the refactor, every file under `codelet/fspec-tui/src/` MUST be under 300 LoC. Specifically the previously over-limit files become:

| File                              | Before | Target action                                                                                |
|-----------------------------------|-------:|----------------------------------------------------------------------------------------------|
| `src/app.rs`                      |    604 | Extract `dispatch` per-Action helpers into `src/app/dispatch.rs`; bootstrap into `src/app/bootstrap.rs` |
| `src/views/agent_repl.rs`         |    519 | Replace with slim `src/views/agent.rs` (≤ 250 LoC); scrollback rendering moved to a helper   |
| `src/views/work_units_list.rs`    |    423 | Replace with `src/views/board.rs` (≤ 250 LoC) — minimal placeholder Kanban skeleton          |
| `src/views/common.rs`             |    463 | Split by responsibility (text wrapping, key formatting, etc.) into submodules                |
| `src/views/root.rs`               |    328 | Rename to `src/views/navigator.rs`, slimmed to ≤ 250 LoC                                     |

A CI assertion (`tests/file_size_rpc012.rs`) enforces the ceiling for the listed files.

---

## 14. Acceptance criteria checklist

This card is "done" when every box can be ticked:

- [ ] `BoardStore` and `AgentViewStore` exist in `codelet/fspec-tui/src/store/`, each under 300 LoC, no `Mutex`/`RwLock`/atomics.
- [ ] `RootView` removed; `Navigator` exists at `src/views/navigator.rs`, ≤ 300 LoC, with `ViewMode { Board, Agent }`.
- [ ] `BoardView` exists at `src/views/board.rs`, minimal placeholder Kanban (decision per Q1), reads from `BoardStore`, emits the four new `Action`s.
- [ ] `AgentView` exists at `src/views/agent.rs`, reads `current_session` from `AgentViewStore`, no internal `active_session` field.
- [ ] New `Action` variants present: `EnterWorkUnit`, `OpenAgentView`, `BackToBoard`, `NavigationTargetSet`.
- [ ] `App::bootstrap` no longer calls `create_session` — only `list_work_units`.
- [ ] `App::dispatch` lazily spawns `create_session(None)` on first `EnterWorkUnit` if no current session.
- [ ] `chunks_rx` subscriber filters via the existing `watch::channel`, fed from `AgentViewStore.current_session`.
- [ ] Every file under `codelet/fspec-tui/src/` is < 300 LoC.
- [ ] `tests/source_shape_stores_rpc012.rs` passes — no forbidden patterns in `store/`.
- [ ] `tests/board_agent_navigation_rpc012.rs` passes against `MockBackend`, `EmbeddedFspecBackend`, and `WebSocketFspecBackend`.
- [ ] `cargo test -p codelet-rpc -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui -p codelet-fspec` is green.
- [ ] Existing insta snapshots for old `agent_repl` / `root` layout are deleted or migrated to new `board` / `agent` snapshot names.
- [ ] `fspec validate` and `fspec validate-tags` pass for any new feature files generated by `fspec generate-scenarios RPC-012`.

---

## 15. References

- Epic review (this session) — [`spec/attachments/RPC-002/review-findings-2026-05-12.md`](../RPC-002/review-findings-2026-05-12.md)
- TS reference — `src/tui/components/UnifiedBoardLayout.tsx`
- TS reference — `src/tui/components/AgentView.tsx`
- TS reference — `src/tui/store/fspecStore.ts`
- TS reference — `src/tui/store/sessionStore.ts`
- RPC-002 dossier — `spec/attachments/RPC-002/07-recommended-architecture.md`
- RPC-002 dossier — `spec/attachments/RPC-002/06-mapping-ink-to-ratatui.md`
- RPC-009 single-task tenere pattern — `spec/attachments/RPC-009/`
- RPC-005 host-supplied runtime — Q9 in `spec/attachments/RPC-005/`
- RPC-008 `FspecBackend` trait surface — `spec/attachments/RPC-008/`

---

## 16. OPEN QUESTIONS (Example Mapping — RED CARDS)

> **These six questions block `fspec generate-scenarios RPC-012`.** Answer each one — either in this thread or via `Fspec answer-question --args '{"_": ["RPC-012", "<idx>"], "answer": "...", "addTo": "rule"}'`. Each answer should be added to either a rule (most common) or an example.

### Q0 — Placeholder BoardView render shape

> Should the placeholder `BoardView` in this slice render the 7 columns at all (even as empty headers), or just show a single flat list inside a `Block` titled "Board" until the rich `UnifiedBoardLayout` port lands?

**Trade-off:** Showing columns proves the `BoardStore.column_units` indexing end-to-end but roughly doubles the placeholder render code (~120 LoC vs ~60). A flat list is faster to ship but defers the column-indexing proof to the downstream slice.

**Recommendation:** Render 7 column headers + a vertical stack of `{id}` per column in muted color (no scroll math, no details strip). This validates `BoardStore.column_units` and matches the eventual `UnifiedBoardLayout` shape topologically.

---

### Q1 — `AgentView` input shape

> Should `AgentView`'s existing single-line `tui_input::Input` be kept as-is for this slice, or replaced with a multi-line input now (since `AgentView` is becoming the long-term home)?

**Context:** Multi-line input is a separate RPC-002 slice already on the dossier. Pulling it forward here would inflate scope (~3–5 points).

**Recommendation:** Keep single-line for this slice. Multi-line lands with the full `AgentView` port.

---

### Q2 — Store ownership location

> Where exactly should the stores live? Options:
> - **(a)** Owned directly by `App` (alongside Compositor + action bus)
> - **(b)** Owned by `RootView`/`Navigator` with `App` holding a `&mut Navigator` reference
> - **(c)** Inside a third `AppState` aggregator struct passed by reference to both `App::dispatch` and `Navigator::render`

**Context:** The TS Zustand pattern colocates state at module top-level. In Rust the choice affects borrow-checker ergonomics inside `App::dispatch`, which must mutate stores AND emit new actions AND spawn backend calls in the same control flow.

**Recommendation:** Option **(a)** — `App` owns the stores. `Navigator::render(&self, area, buf, &BoardStore, &AgentViewStore)` takes immutable refs. This avoids re-borrow conflicts in `dispatch` and keeps `Navigator` near-pure presentation.

---

### Q3 — Lazy session creation test shape

> The lazy `create_session` on first `Enter` — should the test deterministically observe `create_session` called exactly once (the App spawns the future on tokio and the test awaits it directly), or should the test assert only that `AgentViewStore.current_session` transitions from `None` to `Some` within N render ticks?

**Trade-off:** Direct-await is deterministic but requires exposing the in-flight `JoinHandle` from `App` (or a test-only seam). Polled-tick is closer to production behaviour but introduces a tick budget that can flake under load.

**Recommendation:** Direct-await via a test-only `App::next_pending_task() -> Option<JoinHandle<()>>` seam, gated behind `#[cfg(test)]`. Production code path is unchanged.

---

### Q4 — `Tab` keybinding semantics at navigator level

> Should we preserve the existing `Tab` keybinding at the `RootView` level (currently "switch focus between list + REPL"), or repurpose it for view switching in the new navigator (`Tab = Board ↔ Agent`)?

**Context:** The TS TUI uses `Tab` to switch panels INSIDE `BoardView` (board / stash / files) and uses `Enter` / `Shift+Right` for the `BoardView → AgentView` transition. Repurposing it at the navigator level would diverge from the TS contract.

**Recommendation:** Drop `Tab` at the navigator level entirely. `Enter` / `Shift+Right` / `ESC` are the navigator-level transitions; `Tab` is reserved for the eventual in-`BoardView` panel switch (which lands in the rich `UnifiedBoardLayout` slice).

---

### Q5 — `session_attachments` population strategy

> Should the `session_attachments` map in `BoardStore` be populated **automatically** by `App::dispatch` on `Action::SessionCreated` (the App knows `current_work_unit_id` from `AgentViewStore` at that moment), or should attachment be an **explicit** `Action::AttachSession(work_unit_id, session_id)` that views must emit?

**Context:** The TS reference uses an explicit `attachToWorkUnit()` service function. The implicit path is shorter to wire but couples `BoardStore` to `AgentViewStore` through `App::dispatch`'s ordering.

**Recommendation:** **Explicit** — add `Action::AttachSession(String, SessionId)` to the new variant list. `App::dispatch` emits this action itself on `SessionCreated` when `agent_view.current_work_unit_id.is_some()`. Future slices (manual attach via UI, multi-attach, etc.) reuse the same action.

---

## 17. Estimation note

Story points are intentionally not set yet. Once the six questions are answered and `fspec generate-scenarios RPC-012` produces the feature files, estimate on the Fibonacci scale. Expected range: **8 points** (file extraction + navigator rewrite + 2 new stores + integration tests across 3 backends). If answers push scope above 13 the card MUST be split before moving to `testing`.

---

*End of plan.*
