# BUG-179 Research — AgentView input ignores paste & file drop in mux mode

**Date:** 2026-09-12
**Work unit:** BUG-179 (epic: mux)
**Symptom:** In the Rust ratatui TUI, drag-and-dropping a screenshot (or any file) onto the
AgentView message input area, and pasting text (Ctrl+V, bracketed paste) into the input area,
**works in single-view Agent mode but does nothing in mux mode** (`ViewMode::Mux`,
`mux.config().enabled`).

---

## 1. Event pipeline — how it works in single-view mode

### 1.1 Entry point: the run loop

`rust/fspec-tui/src/app/run_loop.rs` — `App::run` drives a `tokio::select!` over the
crossterm `EventStream`. Event variants (crossterm 0.28.1, `src/event.rs:547-563`):

```rust
pub enum Event {
    FocusGained, FocusLost,
    Key(KeyEvent),
    Mouse(MouseEvent),
    #[cfg(feature = "bracketed-paste")]
    Paste(String),
    Resize(u16, u16),
}
```

> **Key fact:** crossterm 0.28 has **NO file-drop event variant**. Terminals either
> (a) deliver a dropped file's path as **typed text** (key events) or (b) as a
> **bracketed paste** (`Event::Paste`) — the fix for both is the same mux routing fix
> (§2), because both funnel into the same gate. See §3 for the per-terminal matrix.

The run loop dispatches (`run_loop.rs:53-66`):

```rust
Some(event) = events.next() => {
    let event = event?;
    match event {
        Event::Paste(text) => { self.handle_paste(&text); }
        Event::Resize(..)  => { self.should_render = true; }
        other              => { self.handle_event(&other); }
    }
}
```

Note `Event::Paste` is **only** ever routed through `App::handle_paste` — it never goes
through `App::handle_event`.

### 1.2 The working paste path (single view)

`App::handle_paste` — `rust/fspec-tui/src/app/events.rs:213-228`:

```rust
pub fn handle_paste(&mut self, text: &str) -> EventResult {
    let result = self.compositor.handle_paste(text);          // Stage 2: modals
    if result.is_consumed() { ... return ...; }
    let event = Event::Paste(text.to_string());
    let nav_result = self.navigator.handle_event(&event, &self.board_store);  // Stage 3
    self.should_render = true;
    nav_result
}
```

`Navigator::handle_event` (`views/navigator.rs:133-144`) matches `active_view`; in
`ViewMode::Agent` it calls `self.agent.handle_event(event)`.

`AgentView::handle_event` — `views/agent/dispatch.rs:50-256`:

```rust
if let Event::Paste(text) = event {
    if let Some(result) = self.handle_hitl_prompt_paste(text) { return result; }
    if let Some(result) = self.handle_exec_stdin_prompt_paste(text) { return result; }
}
...
let before = self.input.value();
let gate = InputGate { block_edits: self.last_is_compacting, suppress_enter: self.last_is_compacting };
let outcome = self.input.handle_event_gated(event, gate);      // ← Paste lands here
self.sync_popups();
match outcome {
    InputEventOutcome::Continued => {
        let after = self.input.value();
        if after != before { self.emit(Action::PendingInputChanged(after)); }
        EventResult::consumed()
    }
    ...
}
```

`MultiLineInput::handle_event_gated` (`views/agent/multiline_input.rs:230-241`) routes
`Event::Paste(s)` to `multiline_input_paste::handle_paste` (`multiline_input_paste.rs:21-31`),
which CRLF-normalizes and inserts verbatim at the cursor (the compacting gate blocks it).

This whole chain is pinned by `spec/features/agent-input-bracketed-paste-routing.feature`
(@RPC-403, @done) and `tests/agent_input_paste_routing_rpc403.rs`.

### 1.3 Why file drop "works" in single-view mode today

There is no dedicated drop handler anywhere in the TUI (verified: no `file-drop`,
`FileDrop`, or URL-parsing code in `rust/fspec-tui/src`). A dropped screenshot works
in single-view Agent mode because the user's terminal synthesizes the drop as input text:
- as a **bracketed paste** (`Event::Paste` with the path / quoted path) → handled by the
  §1.2 chain, or
- as **typed characters** (kitty default, WezTerm, xterm.js terminals) → each char is an
  `Event::Key` → `AgentView::handle_event` → `MultiLineInput` inserts it.

Both only reach the composer because in single-view mode the AgentView is the sole child
of the Navigator and receives **every** event that the compositor doesn't claim.

---

## 2. Root cause — the mux keyboard-isolation gate swallows everything that is not a key

### 2.1 The drop point

`Navigator::handle_mux_event` — `rust/fspec-tui/src/views/navigator_mux_events.rs:23-120`:

```rust
pub(crate) fn handle_mux_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult {
    let is_mouse = matches!(event, Event::Mouse(_));
    if is_mouse {
        let decision = mux_mouse::classify_mouse(&self.mux, event);
        return match decision { ... };              // mouse: dividers / panes
    }
    let Event::Key(key) = event else {
        return EventResult::ignored();              // ← LINE 76-78: PASTE DIES HERE
    };
    ...
}
```

When `active_view == ViewMode::Mux`, `Navigator::handle_event` routes to
`handle_mux_event` (`views/navigator.rs:142`). The method only classifies
`Event::Mouse` and `Event::Key`. Any other variant — **including `Event::Paste`** —
hits the `let Event::Key(key) = event else { return EventResult::ignored(); }` guard
and is dropped.

### 2.2 The full paste trace in mux mode

```
Event::Paste arrives at run loop (run_loop.rs:56)
  └─ App::handle_paste (app/events.rs:213)
       ├─ compositor.handle_paste → Ignored (no modal open)
       └─ self.navigator.handle_event(Event::Paste, …)   (app/events.rs:225)
            └─ Navigator::handle_event → ViewMode::Mux arm (navigator.rs:142)
                 └─ Navigator::handle_mux_event (navigator_mux_events.rs:23)
                      └─ NOT a mouse, NOT a key → EventResult::ignored()   ← paste lost
```

`App::handle_paste` returns that `Ignored` result and the paste is gone. No log line,
no error, no scrollback notice — **silently nothing happens**, exactly the reported symptom.

Contrast with `App::handle_event` (the key path, `app/events.rs:45-145`): after the
Navigator stage it additionally runs (events.rs:117-121):

```rust
if matches!(self.navigator.active_view, ViewMode::Mux) && self.navigator.mux.config().enabled {
    self.sync_mux_focus_to_session();
}
```

`sync_mux_focus_to_session` (BUG-163, `app/dispatch_mux.rs:142-161`) keeps the store's
current session in lockstep with the mux's focused agent pane, performing the RPC-052
draft round-trip via `switch_to_session_index` (`app/dispatch_session_cycle.rs:129-157`:
snapshot outgoing `input_draft` → focus incoming session → restore its draft into the
live `MultiLineInput`). **`App::handle_paste` never calls this**, so even if the paste
reached the pane handler it would be operating on a possibly-stale current session.

### 2.3 Why the paste can't reach the composer any other way in mux mode

- The mux render model (BUG-163, `views/multiplex/render.rs:87-106` +
  `views/agent/pane_render.rs`): there is exactly **ONE** live `MultiLineInput` on the
  single `AgentView` instance; the focused agent pane hosts it, unfocused agent panes
  paint a read-only **ghost draft** (`paint_ghost_input_row`,
  `views/agent/input_area.rs:180-207`).
- The spec's isolation rule (`spec/features/rust-mux-mode.feature`, business rule R2):
  "keyboard input goes only to the focused pane; clicking a pane focuses it and routes
  the click to that pane." The implementation enforces this by forwarding **only
  focused-pane** events through `forward_mux_event_to_focused_pane`
  (`navigator_mux_events.rs:167-191`) → `mux_keys::forward_to_pane`
  (`views/multiplex/keys.rs:60-81`).
- So the paste fix MUST route `Event::Paste` into that same forward-to-focused-pane path
  — there is no per-pane input object to address directly.

### 2.4 Why drag-and-drop also does nothing (or worse) in mux mode

File drop has no dedicated crossterm event (§1.1), so what actually arrives depends on
the terminal:

| Terminal / mode | Drop delivery | What happens in mux today |
|---|---|---|
| kitty (default `copy_url`/shell-escaped) | typed characters (possibly `file://` URL, possibly quoted) | chars → `Event::Key` → `classify_key` → `Forward` → **focused pane only**. If the agent pane is focused: typed into the composer (partially works). If the **Board pane is focused** (the default on fresh mux entry, rule R11): the path characters are consumed as board navigation (`j`/`k`/`h`/`l`/Enter arms in `views/board.rs:147-240`) — silent corruption of board state, no path reaches the composer. |
| WezTerm, xterm.js/VS Code terminals | typed characters (path at cursor) | same as kitty above. |
| tmux (mouse on, 3.2+) | typed characters / paste of path | same as above. |
| Terminals that deliver drops as bracketed paste | `Event::Paste(path)` | **silently swallowed** by the §2.1 gate — the exact reported symptom. |
| Terminals without drop support | nothing | no event at all (not a bug). |

Either way the drop never reaches the composer when it arrives as a paste, and the
typed-character variant is hostage to whichever pane is focused. The unified fix is the
same paste-routing fix (§4): a drop delivered as `Event::Paste` lands in the focused
pane's composer, and a drop delivered as typed text already flows to the focused pane
under R2 (agent pane) — so the user only needs the agent pane focused to drop/paste.

### 2.5 Secondary gap — App::handle_paste lacks the mux focus sync

Even with paste forwarded correctly, `App::handle_paste` does not run
`sync_mux_focus_to_session()` after Navigator routing (see §2.2), so a paste issued
immediately after a click-to-focus (before the next render's `sync_window`) could be
inserted into the outgoing session's draft and snapshotted under the wrong session id by
the next `switch_to_session_index`. Single-view mode has no equivalent because the
Composer's session == the store's current session by construction; mux introduces the
second degree of freedom (mux focus ≠ store current session until synced).

---

## 3. How it SHOULD work in mux mode (target behavior)

1. **Paste (bracketed paste, incl. drop-as-paste):** a paste event delivered while mux
   is active is routed to the **focused pane only** (R2 isolation — same rule as keys):
   - focused pane is an **Agent pane** → insert into the live composer (the shared
     `MultiLineInput` that hosts the focused session's draft), honoring the same gates as
     single-view (HITL freeform/exec-stdin/pause precedence in `dispatch.rs:79-89`,
     RPC-095 compacting `block_edits` gate, CRLF→LF normalization, `PendingInputChanged`
     emission). Ghost panes are never touched.
   - focused pane is the **Board** pane → paste is ignored (the board view has no text
     input; matches single-view Board semantics where `BoardView::handle_event` returns
     `Ignored` for non-key/non-mouse events, `views/board.rs:150-156`).
   - focused pane is Files/Checkpoints → ignored (their `handle_event` returns `Ignored`
     for `Event::Paste` — `views/changed_files/mod.rs:209-215`, checkpoints analogous).
2. **Focus bookkeeping:** after mux paste routing the App performs
   `sync_mux_focus_to_session()` exactly like the key path (`app/events.rs:117-121`),
   so the draft round-trip always targets the right session.
3. **File drop:** no new crossterm plumbing is required — drops surface as either
   `Event::Paste` (fixed by #1) or typed text (already flows to the focused pane under
   R2). Document the expected UX: **drop or paste targets the focused pane**; with the
   default fresh-entry Board focus, the user moves focus to the agent pane
   (Shift+←/→, click-to-focus, or mouse-click inside the agent pane) first.
4. **No behavior change outside mux** (rule R10): when `mux.config().enabled` is false the
   event flow is byte-for-byte unchanged (all existing paste tests stay green).
5. **Ghost-pane safety:** unfocused agent panes remain read-only; a paste must never
   mutate `AgentViewStore.input_draft` of a non-focused session.

---

## 4. Proposed fix design

### 4.1 Minimal fix (recommended)

**A. Route paste through the mux forward path** — `views/navigator_mux_events.rs`,
`handle_mux_event`: add an `Event::Paste` branch alongside the mouse branch, before the
key guard:

```rust
pub(crate) fn handle_mux_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult {
    let is_mouse = matches!(event, Event::Mouse(_));
    if is_mouse { ... unchanged ... }
    // BUG-179: bracketed paste (incl. terminal file-drop delivered as paste) honors
    // the same keyboard isolation as keys — the focused pane only.
    if matches!(event, Event::Paste(_)) {
        return self.forward_mux_event_to_focused_pane(event, board_store);
    }
    let Event::Key(key) = event else {
        return EventResult::ignored();
    };
    ...
}
```

Semantics fall out of the existing forward helper:
- focused **Agent** pane → `agent_view.handle_event(Event::Paste)` → the full
  `dispatch.rs` precedence chain + gates (HITL → exec-stdin → composer), returning
  `Consumed` exactly like single-view mode.
- focused **Board/Files/Checkpoints** pane → their `handle_event` returns `Ignored` →
  the paste is ignored at App level (nothing else consumes pastes — the run loop has no
  paste fallback). Consistent with R2 ("keyboard input goes only to the focused pane").

**B. Mirror the mux focus sync in the paste entry** — `app/events.rs`,
`App::handle_paste`:

```rust
let event = Event::Paste(text.to_string());
let nav_result = self.navigator.handle_event(&event, &self.board_store);
// BUG-179: keep the store's current session in lockstep with the mux's
// focused agent pane — same post-Navigator step as App::handle_event.
if matches!(self.navigator.active_view, ViewMode::Mux) && self.navigator.mux.config().enabled {
    self.sync_mux_focus_to_session();
}
self.should_render = true;
nav_result
```

### 4.2 Non-goals / considerations (open questions for specifying)

- **`file://` URL decoding:** kitty can deliver `file://`-style URLs. Whether dropped
  URLs should be decoded to bare paths (and Windows backslash handling) is a UX
  refinement — the current contract accepts the path as typed/pasted text, identical to
  single-view behavior. Recommend keeping single-view semantics (no decoding) for parity.
- **Drop while Board pane focused:** typed characters are already interpreted as board
  navigation keys (pre-existing single-view Board behavior for typed text; the drop UX
  just needs the agent pane focused). Out of scope for this bug.
- **Newer crossterm with a drop event:** upgrading crossterm (0.29+) is a separate
  dependency decision; nothing in this fix depends on it.

### 4.3 Files touched

| File | Change |
|---|---|
| `rust/fspec-tui/src/views/navigator_mux_events.rs` | `Event::Paste` branch in `handle_mux_event` (~4 lines + comment) |
| `rust/fspec-tui/src/app/events.rs` | `sync_mux_focus_to_session()` call in `App::handle_paste` (~4 lines + comment) |

No changes to `AgentView`, `MultiLineInput`, the compositor, or the mux layout — all
existing gate/precedence logic is reused verbatim.

---

## 5. Test plan (ACDD — write these failing first)

New feature file `spec/features/mux-agent-input-paste-drop-routing.feature`
(tags: `@wip @rust @tui @agent-view @mux @input @bug-179`), mirroring the
`agent-input-bracketed-paste-routing.feature` scenario style. New test file
`rust/fspec-tui/tests/mux_agent_input_paste_bug179.rs` using the shared
`AppTestHarness` (`tests/common/harness.rs`) + mux setup pattern from
`tests/bug163_mux_agent_panes_render_distinct_sessions.rs`
(`app_with_sessions_and_panes` — submit `/mux board agent agent`).

Scenarios → tests (each test maps 1:1 with `@step` comments):

1. **Paste into the focused agent pane inserts into the composer** — mux active with
   Board | Agent | Agent, agent pane focused (focus set via
   `app.navigator_mut().mux.set_focus(i)` / click event through the mux mouse path),
   `app.handle_paste("a\nb\nc")` → `app.navigator().agent.input.value() == "a\nb\nc"`
   AND the store's current session == the focused pane's window session.
2. **Paste while the Board pane is focused is ignored** — same setup, Board focused:
   `handle_paste` returns `Ignored`, input buffer unchanged, board selection unchanged.
3. **Paste after click-to-focus lands in the newly-focused session's draft** — two
   agent sessions in the window; click focuses pane 2 (`Event::Mouse Down` through
   `app.handle_event`), then paste → composer holds the pasted text AND
   `agent_view_store.current_session()` == the clicked pane's window session
   (exercises the §2.5 `sync_mux_focus_to_session` addition in `App::handle_paste`).
4. **Paste never mutates a ghost pane's draft** — pre-seed session B's
   `input_draft`; focus session A's pane; paste; assert session B's `input_draft` is
   unchanged in the store.
5. **Compacting gate preserved in mux** — focused session status `Compacting`,
   `render_once` (so `last_is_compacting` caches), paste → buffer unchanged
   (parity with RPC-403 rule 5).
6. **R10 regression: single-view paste unchanged** — mux disabled, same paste calls as
   `agent_input_paste_routing_rpc403.rs` still pass (re-run existing suite, no code
   change).
7. **Drop-as-paste equivalence** — a drop delivered by the terminal as a bracketed
   paste is the same `Event::Paste` variant; covered by scenario 1 at the routing layer
   (documented as the drop mechanism; no distinct code path exists — see §1.1).

Run: `cargo test -p codelet-fspec-tui --test mux_agent_input_paste_bug179` (scoped —
never unscoped `cargo test` per repo rules).

---

## 6. Evidence index (file:line)

| Fact | Location |
|---|---|
| Run loop routes `Event::Paste` only to `App::handle_paste` | `rust/fspec-tui/src/app/run_loop.rs:53-66` |
| crossterm 0.28.1 `Event` enum has no drop variant | `~/.cargo/registry/src/index.crates.io-*/crossterm-0.28.1/src/event.rs:547-563` |
| Paste → Navigator → Mux arm | `rust/fspec-tui/src/app/events.rs:213-228`, `rust/fspec-tui/src/views/navigator.rs:133-144` |
| **Mux gate drops non-key events (root cause)** | `rust/fspec-tui/src/views/navigator_mux_events.rs:28, 76-78` |
| Focused-pane forwarding (keys today) | `rust/fspec-tui/src/views/navigator_mux_events.rs:167-191`, `rust/fspec-tui/src/views/multiplex/keys.rs:60-81` |
| Post-Navigator mux focus sync (key path only) | `rust/fspec-tui/src/app/events.rs:117-121` |
| `sync_mux_focus_to_session` (BUG-163) | `rust/fspec-tui/src/app/dispatch_mux.rs:142-161` |
| Draft round-trip `switch_to_session_index` | `rust/fspec-tui/src/app/dispatch_session_cycle.rs:129-157` |
| AgentView paste precedence + gates | `rust/fspec-tui/src/views/agent/dispatch.rs:50-256` (paste arms 79-89, gated insert 224) |
| Composer paste insert (CRLF-normalize + compacting gate) | `rust/fspec-tui/src/views/agent/multiline_input.rs:230-241`, `rust/fspec-tui/src/views/agent/multiline_input_paste.rs:21-31` |
| Single shared composer + ghost panes (mux render model) | `rust/fspec-tui/src/views/multiplex/render.rs:87-106`, `rust/fspec-tui/src/views/agent/pane_render.rs:53-179`, `rust/fspec-tui/src/views/agent/input_area.rs:180-207` |
| R2 keyboard-isolation rule + R10 no-change-outside-mux | `spec/features/rust-mux-mode.feature` (business rules R2, R10, R11) |
| Existing paste acceptance criteria (single view) | `spec/features/agent-input-bracketed-paste-routing.feature`, `rust/fspec-tui/tests/agent_input_paste_routing_rpc403.rs` |
| Mux test setup pattern (enable mux in tests) | `rust/fspec-tui/tests/bug163_mux_agent_panes_render_distinct_sessions.rs:61-84`, `rust/fspec-tui/src/views/multiplex/mod.rs:67` (`enable_default`) |
| Board view ignores non-key/non-mouse events | `rust/fspec-tui/src/views/board.rs:147-156` |
| ChangedFiles ignores Paste | `rust/fspec-tui/src/views/changed_files/mod.rs:209-215` |

## 7. Related work units

- **BUG-163** — mux agent panes render distinct window sessions; introduced the shared
  composer + ghost-draft model and `sync_mux_focus_to_session` (the sync this bug's fix
  must reuse on the paste path).
- **MUX-001 / MUX-002** — mux grid + keyboard isolation (R2) and focus cycling; the
  isolation trap this bug breaks through for paste.
- **RPC-403** — bracketed-paste routing into the agent input (the single-view chain the
  fix reuses).
- **RPC-019** — MultiLineInput contract (paste inserts verbatim, auto-grow).
- **BUG-164 / BUG-165 / BUG-174 / BUG-175** — surrounding mux robustness fixes
  (grid retention, board-pane Esc, empty-agent floor, enabled-flag leak) — context for
  why the mux routing surface must stay conservative.
