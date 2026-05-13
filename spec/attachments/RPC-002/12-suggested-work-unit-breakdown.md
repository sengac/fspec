# 12 — Suggested Work Unit Breakdown

This document proposes a child-story breakdown for the RPC-002 epic.
**It is a recommendation, not a commitment.** Final breakdown happens
during Example Mapping when the card moves to `specifying`.

The slices are ordered by dependency. Each is sized for the Fibonacci
estimation scale defined in the project workflow doc; estimates are
rough indications, to be refined per slice.

---

## Slice graph

```
01 (Foundation: app shell + Compositor + 1 dialog)
       |
       v
02 (Mouse + bracketed paste + tarpc transport)
       |
       v
03 (VirtualList: standard mode)
       |     \
       v      \
04 (VirtualList: 05 (Dialog hierarchy:
    lazy + group +    Confirmation, Three-Button,
    advanced features)Status, Role, etc.)
       |
       v
06 (MultiLineInput on tui-textarea)
       |
       v
07 (BoardView)
   (AgentView)
   (CheckpointViewer)
   (FileSearchPopup)
   (UnifiedBoardLayout)
   ...
```

---

## Slice 01 — Foundation: App shell + Compositor + ESC dialog

**Estimate:** 5 points

**Goal:** prove the architecture by getting an app off the ground.

**Acceptance:**
- New `fspec-tui` Rust crate scaffolded from ratatui-org
  `templates/component`.
- `Priority` enum + `EventResult` + `Component` trait defined.
- `Compositor` implementation with priority sort, FIFO tiebreak,
  `is_active` gating, callback-deferred mutations, `pop()` /
  `remove(id)`.
- `App` struct with `tokio::select!` over events / ticks / actions /
  cancel.
- One placeholder `HelloComponent` rendering centered text at
  `Priority::Background`.
- One `Dialog` wrapper (using `tui-popup` from ratatui-org/tui-widgets,
  per resolved Q5) at `Priority::Critical` triggered by `?` key.
- ESC closes the dialog.
- Unit tests for Compositor (12 cases per the test plan in doc 09).
- Snapshot test of dialog rendering via `TestBackend` + insta.

**Outputs:**
- `compositor.rs`, `dialog.rs`, `app/mod.rs`, `theme.rs`.
- The first feature file: e.g. `compositor-priority-routing.feature`.

**Dependencies:** none.

---

## Slice 02 — Mouse + bracketed paste + tarpc transport

**Estimate:** 8 points

**Goal:** Wire up the foundation to fspec via tarpc, enable mouse and
paste.

**Acceptance:**
- `EnableMouseCapture` + `EnableBracketedPaste` on startup; cleanup on
  exit (via `Drop` and a panic hook).
- `MouseTrackingToggle` utility with 5-second debounce timer.
- `trait FspecBackend` defined to mirror the JS API surface (subset
  needed for the next slice).
- `EmbeddedBackend` and `WebSocketBackend` implementations stubbed (or
  one with a stub).
- `App` accepts an `Arc<dyn FspecBackend>`.
- One trivial integration: clicking somewhere in the placeholder UI
  calls a backend method and an action loops back through the bus.

**Dependencies:** Slice 01.

---

## Slice 03 — VirtualList: standard mode (non-lazy, non-grouped)

**Estimate:** 8 points

**Goal:** the simplest VirtualList that covers ~70 % of consumers.

**Acceptance:**
- `VirtualList<T: ListItem>` struct with builder.
- Item virtualization (slice items array based on `scroll_offset`).
- ratatui core `Scrollbar` widget rendered to the right.
- Up / Down / PageUp / PageDown / Home / End / Enter keyboard nav.
- Mouse-wheel scroll (no velocity yet; that's slice 04).
- `is_focused` / `is_active` gating.
- Selection mode = Item (single).
- `on_select(item, idx) -> Action` callback.
- Empty state ("No items").
- Unit tests for navigation; snapshot tests for rendering.

**Dependencies:** Slice 01.

---

## Slice 04 — VirtualList: lazy + group + scroll-mode + velocity + text-selection toggle

**Estimate:** 13 points (consider splitting if discovery says 21)

**Goal:** feature parity with the Ink VirtualList.

**Acceptance:**
- Lazy mode: `trait ListItems` with `len() + range(start, end) -> Vec<T>`.
- Group selection: `Grouped` trait with `group_id(index)`; navigation
  jumps groups; selection highlights all items in a group.
- Group selection preservation across mutations.
- `SelectionMode::Scroll` (move viewport without changing selection).
- `scrollToEnd` auto-stick; `userScrolledAway` detection.
- Mouse-wheel velocity acceleration (150 ms / cap 5).
- Native text-selection toggle (mouse press disables, release re-enables,
  5 s debounce, integrates with `MouseTrackingToggle`).
- `selectionRef` equivalent (parent-readable selection).
- All VirtualList tests in `08-virtuallist-port-spec.md` pass.

**Dependencies:** Slice 03.

**Risk note:** if the spike from Q5 (rat-event) or Q6 (tui-widget-list)
shows incompatibilities, this slice expands. Plan a 1-day spike at the
start.

---

## Slice 05 — Dialog hierarchy: Confirmation + Three-Button + Status + Role

**Estimate:** 8 points

**Goal:** port the dialog family.

**Acceptance:**
- `Dialog` wrapper (already in slice 01) gains result-channel support
  (oneshot::Sender<T>).
- `ConfirmationDialog` with Tab cycling, default focus, Enter on
  confirm, ESC on cancel.
- `ThreeButtonDialog` with three-way Tab.
- `StatusDialog` (auto-close on action or after timeout).
- `RoleDialog` (form with embedded VirtualList of roles).
- `CreateSessionDialog` (multi-field form).
- Tests + snapshot per dialog.

**Dependencies:** Slice 01 + Slice 03.

---

## Slice 06 — MultiLineInput on tui-textarea

**Estimate:** 13 points

**Goal:** port the composer.

**Acceptance:**
- `MultiLineInput` wraps `tui-textarea`.
- Submit on Enter; newline on Shift+Enter.
- History persistence (`~/.fspec/tui-history.jsonl`).
- Up at top of input pulls history; Down at bottom advances or clears.
- `/` opens slash command palette.
- `@` opens file mention popup (uses `nucleo` for fuzzy match).
- Bracketed paste forwarded to textarea.
- Auto-grow up to max_height.
- Tests (~15 cases).

**Dependencies:** Slice 01 (Compositor for popup-on-popup).

---

## Slice 07+ — Consumer ports (parallelizable)

Each of these is independent once Slices 01-06 land:

| Component | Estimate | Notes |
|---|---|---|
| `BoardView` | 13 | Multi-column Kanban with focus traversal; uses VirtualList per column |
| `AgentView` | 13 | Streaming chat with tool-call cards; lazy VirtualList; HistoryCell-like trait |
| `CheckpointViewer` | 8 | Diff viewer with bordered VirtualList |
| `UnifiedBoardLayout` | 8 | Composite layout |
| `ConversationInputArea` | 8 | Composer + status + agent badges |
| `FileSearchPopup` | 5 | Already covered by MultiLineInput slice if we want |
| `SlashCommandPalette` | (covered by slice 06) | |
| `ChangedFilesViewer` | 5 | List + filesystem watcher integration |
| `ProviderSettingsScreen` | 8 | Multi-step form |
| `ThinkingLevelDialog` | 3 | Slider |
| `AttachmentDialog` | 5 | Use `ratatui-explorer` |
| `BlocklistListView` | 3 | Simple list with item actions |
| `WorkUnitMetadata` | 2 | Read-only renderer |
| `WorkUnitAttachments` | 2 | Read-only renderer |
| `KeybindingShortcuts` | 1 | Footer hint bar |
| `RoleBanner` | 1 | Header |
| `SessionFooter` | 2 | Footer |
| `SessionHeader` | 3 | Header |
| `ThinkingIndicator` | 2 | Spinner via `throbber-widgets-tui` |
| `CheckpointStatus` | 2 | Status pill |
| `WatcherListView` | 5 | Watcher CRUD |
| `MCPServerSettings` | 5 | Config screen |
| `CustomModelFormView` | 5 | Inline form |
| `CopilotOauthRender` | 5 | OAuth flow |

**Total slice 07+ estimate:** ~115 points across ~24 child stories.

---

## Cross-cutting work units (separate from UI port)

| Work unit | Estimate | Notes |
|---|---|---|
| tarpc service definition (mirror `useApi`) | 5 | One-off |
| Embedded transport implementation | 8 | tokio in-process channel |
| WebSocket transport implementation | 8 | tarpc WS |
| Theme system (light + dark) | 5 | |
| Keybinding registry | 3 | |
| Persisted UI state (window size, last-opened pane) | 3 | |
| Cross-platform startup / teardown (panic hook for terminal restore) | 3 | |
| `fspec-tui` standalone binary entry point | 2 | |
| Insta snapshot harness + CI runner | 3 | |
| Docs + ADR for the Compositor decision | 2 | |

---

## Recommended ordering for the first sprint

> **Note (2026-05-08):** The Q-block is now resolved (see doc 11). The
> first sprint starts directly at Slice 01.

1. **Slice 01 (foundation).**
2. **Spike: confirm `tui-widget-list` accepts external selection state**
   (1 day). If yes, proceed to slice 03 and use it. If no, plan extra
   LoC for slice 03.
3. **Slice 02 (mouse + transport).**
4. **Slice 03 (VirtualList standard mode).**

Past that point the work parallelises across multiple developers, with
slices 04 / 05 / 06 cleared dependently and slice 07+ in parallel
streams.

---

## Definition of done for the epic

RPC-002 is **done** when:

- All Ink components in `src/tui/` and `src/components/` have a Rust
  equivalent.
- The Rust binary passes the same `microsoft/tui-test` E2E tests that
  the Ink binary passes (where applicable).
- The legacy Ink TUI is removed from the default invocation path.
- Coverage for new Rust code is at least at parity with the Ink TUI's
  coverage (per fspec coverage tracking).
- `spec/foundation.json` records the architecture decisions.
- A migration ADR is checked in to `spec/architecture/`.
