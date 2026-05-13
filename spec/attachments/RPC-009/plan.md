# RPC-009 — Basic frontend: work-units list + agent REPL (the FIRST visible UI)

**Parent:** RPC-002
**Predecessor:** RPC-008 (app shell + backend trait)
**Successor:** RPC-010 (binary entry points)

## What we want

The first user-visible iteration of the new ratatui frontend. Two
panes, no fancy widgets, no theming work, no virtualisation — just
proof that real work-unit data and a real agent REPL flow through the
dual-transport seam end-to-end. After this card the same compiled crate
can be driven by either an embedded `SharedFspecService` or a
`WebSocketFspecBackend`, and both produce an identical experience.

## Why this card

Until users can actually see something happen, the whole architecture
is invisible. RPC-009 is the smallest screen that demonstrates:
- A work-unit list reading from a real watcher and updating live.
- An agent REPL that creates a session, sends input, and renders
  streaming chunks.
- Both behaviours work the same on either transport.

Anything more — VirtualList polish, modals, board view, slash commands
— is RPC-002 Slice 03+ and lives in its own card.

## Existing RPC-005..008 artifacts this card builds on

RPC-009 is pure UI on top of the trait + crates already in place. It
must NOT call into NAPI, into `codelet-core`, into `tarpc`, or into
`tokio_tungstenite` directly — every interaction goes through the
`FspecBackend` trait introduced in RPC-008.

| Existing artifact | Path / origin | How RPC-009 uses it |
|---|---|---|
| `FspecBackend` trait (`list_work_units`, `list_sessions`, `create_session`, `send_input`, `interrupt`, `work_units_rx`, `chunks_rx`, `logs_rx`) | `codelet/fspec-tui/src/transport/mod.rs` (RPC-008) | Both views (`work_units_list.rs` and `agent_repl.rs`) take `Arc<dyn FspecBackend>` in their constructor. They never know which transport is behind it. |
| `App`, `Compositor`, `Component` trait, `EventResult`, `Priority` enum | `codelet/fspec-tui/src/app/{mod,compositor,event,action}.rs` (RPC-008) | The two new views and the root layout implement `Component` per the existing pattern. The `?`-help dialog established in RPC-008 is reused — RPC-009 only swaps its body text. |
| `Action` enum | `codelet/fspec-tui/src/app/action.rs` (RPC-008) | Extended with `LoadWorkUnits`, `WorkUnitsLoaded(Vec<WorkUnitInfo>)`, `SessionCreated(SessionId)`, `ChunkReceived(SessionId, StreamChunk)`, `InputSubmitted(String)`, `Interrupt`. `Action::Quit` already exists. |
| `WorkUnitInfo`, `SessionId`, `SessionInfo`, `StreamChunk` | `codelet/rpc-types/src/lib.rs` (RPC-005 + RPC-007 lifts) | Used directly in the two views; no UI-side wrappers, no parallel types. |
| `EmbeddedFspecBackend::new(handle, service)` | `codelet/fspec-tui/src/transport/embedded.rs` (RPC-008) | The embedded smoke test in this card constructs an `Arc<SharedFspecService>` from the real `WorkUnitsWatcher` (RPC-006) against a temp workspace, wraps it in `EmbeddedFspecBackend`, and passes that to `App::new`. |
| `WebSocketFspecBackend::connect(url)` | `codelet/fspec-tui/src/transport/websocket.rs` (RPC-008) | The WS smoke test spawns `codelet-rpc-server` (the existing RPC-005 binary, updated in RPC-006 to take a `--workspace` flag), reads the port off stdout per the contract from `codelet/rpc-server/src/main.rs`, and connects via `WebSocketFspecBackend`. |
| `codelet-rpc-server` binary stdout port-line contract | `codelet/rpc-server/src/main.rs` | The WS smoke test reuses the same harness pattern already proven in `codelet/rpc-server/tests/websocket_transport.rs::spawn_rpc_server` (the `ChildGuard` RAII wrapper + `BufReader::read_line` for the port). Lift to `codelet/fspec-tui/tests/common/` for reuse. |
| Mock backend pattern | (does not yet exist) | Add `tests/common/mock_backend.rs` to `codelet/fspec-tui` implementing `FspecBackend` against in-memory state. Pattern mirrors `codelet/rpc/src/lib.rs::SharedFspecService` (atomic counters, scripted responses) so the snapshot tests are deterministic. |
| `insta` + `TestBackend` snapshot harness | established by RPC-008's dialog snapshot test | Reused for the three snapshot checkpoints (initial / after first chunk / after submit). |
| Reserved-variant + source-shape regression tests | `codelet/rpc-embedded/tests/architecture_invariants.rs`, `codelet/rpc-server/tests/websocket_transport.rs` | Untouched by this card; must still pass. CI assertion: `cargo test -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui` is green at end of card. |
| Vitest smoke | `src/__tests__/napi-workunitinfo-shape.test.ts` | Untouched; must still pass. |

## Architecture conformance with RPC-002

This card stays inside the bare-ratatui + Compositor + `tui-popup`
envelope established in RPC-008 and respects every RPC-002 deferral.
**Crucially, no widget added in this card requires a future Slice
deliverable** — virtualisation, multi-line input, slash commands, file
mentions, mouse-tracking toggle, and theming-beyond-default all stay
deferred.

| RPC-002 decision / pattern | Source | RPC-009 obligation |
|---|---|---|
| Q1: alt-screen mode | doc 11 §Q1 | Same `App::run()` from RPC-008. The REPL is rendered INSIDE the alt-screen canvas (NOT into native scrollback like Codex — see doc 04 §1 and doc 01 §"Codex is not the model to copy"). |
| Bare ratatui (Q3) + ~30-LoC Compositor (Q2) | doc 11 §Q2/Q3 | Two new components; both implement the existing `Component` trait. No new framework. |
| Q4 deferred (no `tui-textarea` yet) | doc 11 §Q4, doc 12 §Slice 06 | Single-line input box only. Backed by **`tui-input`** (doc 03 §A.7, headless cursor + buffer state machine, single-line) — chosen over a hand-roll because it is the lightest crate that gives us cursor + backspace + delete + left/right correctly across UTF-8 boundaries. **NOT** `tui-textarea` (Slice 06), **NOT** `reedline` (REPL, not a ratatui widget). |
| Q5 resolved: `tui-popup` for dialogs | doc 11 §Q5 | Help dialog body text changes from RPC-008 placeholder to one-line-per-key; same `Dialog` widget, no new dialog code. |
| Q6 deferred (no `tui-widget-list` yet) | doc 11 §Q6, doc 12 §Slice 03/04 | Work-units list is rendered via ratatui core **`List` + `ListState`** (doc 03 §A.1 row "ratatui core List/Table — windowed only"). Acceptable for the current ~hundreds-of-work-units corpus; full virtualisation arrives in Slice 03. NOT `tui-widget-list`. |
| Group selection / scroll-vs-item / scrollToEnd / mouse-wheel velocity / native text-selection toggle | doc 12 §Slice 04 | All deferred. The list is a flat single-selection list. |
| REPL scrollback rendering | doc 04 §1 (Codex anti-pattern), doc 05 row "tenere", doc 02 §3.5 | Confirmed pattern: `Paragraph::new(Text::from(scrollback_lines)).scroll((y_offset, 0)).wrap(Wrap { trim: false })` against an alt-screen `Rect`. Stick-to-bottom flag is a plain `bool` field on the component (tenere uses an `AtomicBool`; we don't need atomicity since the field is only mutated on the App task — doc 05 §tenere). NOT a virtualised list, NOT a custom `BubbleList`, NOT a `HistoryCell` trait — those are RPC-002 follow-ons. |
| Layout (Yoga → Constraint) | doc 06 §Layout | Two-pane split: `Layout::horizontal([Constraint::Length(32), Constraint::Min(0)])`. Right pane vertical split: `Layout::vertical([Constraint::Min(0), Constraint::Length(3)])` (scrollback + 3-row input box). Footer hint bar: `Constraint::Length(1)` at the very bottom of the root layout. |
| `mpsc::UnboundedSender<Action>` action bus from `templates/component` | doc 03 §D, doc 07 §4 | Every async observation (broadcast subscriber tasks) emits `Action::*` and never touches component state directly. |
| Per-event `is_active()` gating from `useInputCompat` (doc 02 §1.4) | doc 09 §A.6 | Pane that is NOT focused returns `EventResult::Ignored(None)` from `handle_event` while still returning true from `is_active`. Tab cycling toggles a `focused: bool` field on each pane and the App reads it for footer styling — no new compositor surface. |

## Scope (deliberately tiny)

### Layout

```
┌──────────────────────┬───────────────────────────────────────────┐
│ Work Units           │ Agent: <session-id> [role: ...]           │
│ ──────────           │ ──────────                                │
│ AUTH-001 done        │ user> hi                                  │
│ AUTH-002 implementing│ assistant> Hello! How can I help?         │
│ ...                  │ ...                                       │
│                      │                                           │
│                      │ ┌─ input ────────────────────────────┐    │
│                      │ │ > _                                │    │
│                      │ └────────────────────────────────────┘    │
└──────────────────────┴───────────────────────────────────────────┘
```

- Left pane: vertical list of work units (id + status), j/k or Up/Down
  navigation, Enter does nothing visible (selection only). Live-updates
  via `work_units_rx`.
- Right pane: scroll-back area showing user/assistant messages, plus a
  single-line input box at the bottom. Enter sends, Ctrl+C interrupts,
  Ctrl+D quits.
- Footer (bottom row): keybinding hints `?`-help, `q`-quit,
  `Tab`-switch-pane.

### Components added to `codelet/fspec-tui`

- `views/work_units_list.rs` — naive list rendered via ratatui core
  `List` + `ListState` + `Block::default().borders(Borders::ALL)`
  (doc 03 §A.1, doc 06 §Layout). NO virtualisation, NO group selection
  — those are RPC-002 Slices 03/04. Reads `backend.work_units_rx()`.
  Initial fetch via `backend.list_work_units()`. Single-item selection
  via `ListState::select(Some(idx))`. Item rendering: each work unit is
  a `ListItem::new(format!("{} {}", id, status))` styled by status
  (uses the existing `Theme` from RPC-008).
- `views/agent_repl.rs` — two sub-areas: a scrollback area and an input
  area.
  - Scrollback: `Vec<RenderedChunk>` field (where `RenderedChunk` is a
    pre-rendered `Vec<Line<'static>>` keyed by chunk seq — an early
    nod to oatmeal's `BubbleCacheEntry` from doc 05 §oatmeal so we
    don't re-format chunks on every frame, but minus any caching
    machinery — just owned `Lines`). Rendered via
    `Paragraph::new(Text::from_iter(...)).scroll((y_offset, 0))`.
    `stick_to_bottom: bool` field (tenere pattern, doc 05 §tenere): true
    by default; flipped to false when user scrolls up; flipped back to
    true when scroll position reaches the end.
  - Input: `tui_input::Input` field rendered via
    `Paragraph::new(input.value()).block(Block::default().borders(Borders::ALL))`.
    Cursor positioned via `frame.set_cursor_position` based on
    `input.visual_cursor()` (doc 03 §A.7 row "tui-input").
  - Reads `backend.chunks_rx()` and filters by `session_id`.
  - Submit on `KeyCode::Enter` calls `backend.send_input(...)` and
    clears the input via `input.reset()`.
  - Ctrl+C on the input pane sends `Action::Interrupt` (NOT a quit —
    that's footer-level Ctrl+D).
- `views/root.rs` — two-pane horizontal layout, Tab cycles `focused`
  between the two pane structs. Footer hint bar (1 row) renders
  keybinding hints via styled `Spans` — no `tui-prompts`, no
  `throbber-widgets-tui` (those are deferred).
- New `Action` enum entries (added to `app/action.rs`, building on
  RPC-008's `Action::Quit`): `LoadWorkUnits`,
  `WorkUnitsLoaded(Vec<WorkUnitInfo>)`, `SessionCreated(SessionId)`,
  `ChunkReceived(SessionId, StreamChunk)`, `InputSubmitted(String)`,
  `Interrupt`, `FocusNext`.

### Bootstrap behaviour

On `App::run()`:
1. `backend.list_work_units()` → seed left pane (also primes
   `stick_to_bottom = true` and selects index 0).
2. `backend.create_session(None)` → seed right pane.
3. Spawn three subscriber tasks reading the backend's broadcast
   receivers and converting messages into actions on the existing
   `mpsc::UnboundedSender<Action>` bus (doc 07 §4):

   ```rust
   tokio::spawn({
       let tx = action_tx.clone();
       let mut rx = backend.work_units_rx();
       async move {
           while let Ok(units) = rx.recv().await {
               let _ = tx.send(Action::WorkUnitsLoaded(units));
           }
       }
   });
   // analogous spawns for chunks_rx and logs_rx
   ```

   Pattern matches doc 06 §Async-work and the `templates/component`
   architecture from doc 03 §D.

### Dependencies added to `codelet/fspec-tui`

```toml
tui-input = "0.10"   # single-line input (Q4 deferred; tui-textarea is RPC-002 Slice 06)
```

That is the ONLY new dependency. No `tui-widget-list`, no
`tui-textarea`, no `tui-tree-widget`, no `throbber-widgets-tui`.

### Help dialog

Replace the RPC-008 placeholder text with one-line-per-key help: `j`,
`k`, `Tab`, `?`, `q`, `Enter`, `Ctrl+C`. Still uses the same
`Priority::Critical` dialog component — no new dialog widget needed.

### Tests

- `MockFspecBackend` exercise: drive the app with a scripted sequence of
  `WorkUnitsLoaded` and `ChunkReceived` events, snapshot the buffer at
  three points (initial, after first chunk, after submit).
- Embedded smoke: real `SharedFspecService` + real
  `WorkUnitsWatcher` against a temp workspace; assert list updates when
  the file changes.
- WS smoke: real `rpc-server` binary spawned by the test, real WS
  client; assert same observable behaviour.

## Out of scope

- `tui-textarea`-based MultiLineInput — RPC-002 Slice 06.
- VirtualList — RPC-002 Slices 03–04.
- BoardView — RPC-002 Slice 07.
- Theming, status bar, role banner, multi-session tabs.
- Slash commands, file mentions, attachments.
- Mouse support beyond what crossterm gives us for free.

## Acceptance — done when

1. Both panes render with real data.
2. Live work-units updates work on both transports.
3. Agent REPL accepts input, displays streaming responses, handles
   interrupt, on both transports.
4. Snapshot tests pin the look of the screen at three checkpoints.
5. Build is clean; no NAPI regression.

## Estimate guidance

8 points. Naive widgets, but the bootstrap orchestration (three
broadcast subscribers, action bus wiring, focus cycling) is fiddly.
