# AST research — MENU-010: chip selected-highlight scoped to surface

Research performed with AstGrep before implementation. Question: where is the
per-chip `active` flag produced, consumed, and painted?

## Key entities

| Entity | File | Role |
|--------|------|------|
| `MenuChip` struct (`active: bool` field) | rust/fspec-tui/src/components/menu_bar/chips.rs | Data model; `active` drives the selected-item (inverse-video) highlight |
| `ChipInput` struct | rust/fspec-tui/src/components/menu_bar/chips.rs | Input to `build_chips`; carries `active` per session |
| `chip_inputs(store, active)` | rust/fspec-tui/src/views/board/menu_snapshot.rs | **Producer** — was `active: current.is_some_and(\|c\| c == id)` (MENU-006); now `active: false` for all surfaces |
| `mark_active_chip(chips, active, session)` | rust/fspec-tui/src/views/board/menu_snapshot.rs | **New** — marks one chip by position in the painted list; no-op when session not painted |
| `zone_b_and_chips(store, active, clock_ms)` | rust/fspec-tui/src/views/board/menu_snapshot.rs | Builds Zone B cells + chips from `chip_inputs` |
| `build_snapshot` (board) | rust/fspec-tui/src/views/board/menu_snapshot.rs | Board surface snapshot — marks **no** chip |
| `agent_bar_snapshot` | rust/fspec-tui/src/views/agent/menu_render.rs | Agent surface snapshot — marks `store.current_session()` |
| `build_snapshot` (mux) | rust/fspec-tui/src/views/multiplex/menu_snapshot.rs | Mux top-bar snapshot — marks `layout.focused_session_id()` (None for non-Agent panes) |
| `focused_cell` (painter) | rust/fspec-tui/src/components/menu_bar/paint.rs (~line 127) | **Consumer** — `if chip.active { inverse style }` ORed with ring focus (`MenuFocus::ZoneB`). UNCHANGED. |
| `MultiplexLayout::focused_session_id()` | rust/fspec-tui/src/views/multiplex/window.rs:84 | Focused Agent pane's window session; None for Board/Files/Ckpts panes |
| `AgentViewStore::current_session()` | rust/fspec-tui/src/store/agent_view.rs:132 | Store's current session — persists across view flips (root of the stale-highlight bug) |

## Call graph (chip `active` flag)

```
chip_inputs ──► zone_b_and_chips ──► build_chips ──► MenuChip.active
     ▲                ▲
board build_snapshot  agent_bar_snapshot ──► mark_active_chip(store.current_session)
multiplex build_snapshot ──► mark_active_chip(layout.focused_session_id())
                                   │
                                   ▼
                          painter focused_cell: highlight ⇔ ring_focus OR chip.active
```

## Findings

1. **Single painter, three surfaces** (MENU-001): board/agent/mux all render
   through `components/menu_bar/paint.rs`; the only per-surface difference
   before MENU-010 was Zone A items. The `active` flag was computed identically
   for all three surfaces in the shared `chip_inputs`.
2. **Root cause of the stale highlight**: `store.current_session()` survives a
   Board→Agent→Board flip, so the board bar kept painting the last active
   agent's chip inverse.
3. **Fix shape**: keep the painter untouched (ring path `MenuFocus::ZoneB` is
   surface-independent per R3); scope the flag at snapshot-build time — the
   surface decides which session, if any, is marked. `mark_active_chip` is a
   no-op when the session is not in the painted (non-Cleared) chip list, so
   closed sessions never resurrect a highlight.
4. **Mux parity**: in mux mode the focused Agent pane's window session (not the
   store's current session) is the "active agent"; `sync_window` runs during
   the first render, so `focused_session_id()` is live before the bar paints.
5. **Ring geometry**: only the FOCUSED pane refreshes geometry/chip count
   (BUG-163 rule); agent-view ring math reads the chip count from the last
   focused render — tests must paint a frame before exercising the ring.
