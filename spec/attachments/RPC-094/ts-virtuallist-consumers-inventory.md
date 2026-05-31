# RPC-094 — TypeScript VirtualList consumers vs Rust port status

This inventory documents every TypeScript file that uses
`src/tui/components/VirtualList.tsx` (689 LoC) and maps it to its
**Rust ratatui port equivalent** (or notes the gap).

Sources discovered via `Grep VirtualList src/tui/**`. Confirmed
2026-05-29.

---

## 1. Direct VirtualList consumers in TypeScript

| TS source | Role | Lines | Rust port | Status |
|---|---|---|---|---|
| `src/tui/components/AgentView.tsx` | Streaming chat scrollback | ~6 000 LoC | `codelet/fspec-tui/src/views/agent/scrollback.rs` + `agent.rs` | **PARTIAL — RPC-094 closes the gap** |
| `src/tui/components/CheckpointViewer.tsx` | Checkpoint list + diff preview | ~? | _not ported_ | Future card (Rust TUI does not yet expose `/checkpoints` view) |
| `src/tui/components/TurnContentModal.tsx` | Turn detail / select-mode modal | ~? | _not ported_ | Future card (Rust TUI has no select-mode modal yet) |
| `src/tui/components/MultiLineInput.tsx` | Multi-line composer | 308 LoC | `codelet/fspec-tui/src/views/agent/multiline_input.rs` | **N/A** — TS MLI imports VirtualList only for the slash-command hint dropdown which is owned by `slash_command_popup.rs` in Rust and was already migrated by RPC-028 |
| `src/tui/components/FileDiffViewer.tsx` | Per-file diff scroll viewer | ~? | _not ported_ | Future card |
| `src/tui/components/ChangedFilesViewer.tsx` | Changed-files browser | ~? | _not ported_ | Future card |

**Indirect imports (utility / type re-exports — NOT consumers):**

| TS source | Notes |
|---|---|
| `src/tui/types/conversation.ts` | re-exports VirtualList item types |
| `src/tui/types/provider.ts` | re-exports VirtualList item types |
| `src/tui/utils/conversationUtils.ts` | helpers for VirtualList items |
| `src/tui/utils/turnSelection.ts` | helpers for VirtualList items |
| `src/tui/utils/textWrap.ts` | helpers for VirtualList items |
| `src/tui/input/__tests__/input-priority-propagation.test.tsx` | input wiring test |

---

## 2. Behaviour surface of `VirtualList.tsx`

The TypeScript widget bundles **all** of the following into a single
component. Any Rust port that claims VirtualList parity MUST cover the
subset its consumer relies on.

| Surface | TS source anchor | Rust port location | Status |
|---|---|---|---|
| **Item virtualization** (render only the visible window) | `VirtualList.tsx` `visibleItems` | `scrollback.rs::render_count_visited` | ✅ RPC-019 |
| **`scrollToEnd` + `userScrolledAway`** stick-to-bottom | `VirtualList.tsx` `useEffect`s | `ScrollState::stick_to_bottom` | ✅ RPC-019 |
| **Keyboard PgUp / PgDn / Home / End** | `VirtualList.tsx` keyboard branch | `views/agent/dispatch.rs` (Action::ScrollbackPageUp / PageDown) | ✅ RPC-019 / RPC-024 — **End/Home for scrollback NOT wired separately, only PageDown** |
| **Keyboard Up / Down (1 line)** | TS forwards arrows when input cursor at first/last line — AgentView routes them to the scrollback (`AgentView.tsx:4373`) | `views/agent/dispatch.rs` | ❌ **GAP — covered by RPC-094** |
| **Mouse wheel ScrollUp / ScrollDown** | `VirtualList.tsx` `useInputCompat` mouse arm | `views/agent/mouse_dispatch.rs` | ❌ **GAP — covered by RPC-094** |
| **Wheel velocity 1×–5× ramp (≤150 ms)** | `AgentView.tsx:4435-4458` | `components/scroll_viewport.rs::WheelVelocity` exists since RPC-028 — **not consumed by scrollback yet** | ❌ **GAP — covered by RPC-094** |
| **Visual scrollbar gutter** (■ thumb / │ track) | `VirtualList.tsx::Scrollbar` | _none_ — Rust scrollback paints zero gutter | ❌ **GAP — covered by RPC-094** |
| **Native text-selection toggle (?1000l / 5 s timeout)** | `VirtualList.tsx` button-down arm + `useTerminalSize` | RPC-023 wires the toggle GLOBALLY in `App` shell — covers scrollback already | ✅ RPC-023 |
| **`group_by` / `group_padding_before`** | `VirtualList.tsx` group nav | _none_ (scrollback is flat) | N/A for AgentView; future CheckpointViewer port may need it |
| **Lazy mode `getItems` + `itemCount`** | `VirtualList.tsx` `useMemo` | _none_ (scrollback is in-process) | N/A for AgentView |
| **SelectionMode::Item vs Scroll** | `VirtualList.tsx` selection branch | scrollback is Scroll-only | N/A for AgentView; future CheckpointViewer needs Item mode |

---

## 3. RPC card history relevant to this work

| Card | Status | Coverage |
|---|---|---|
| **RPC-002 §8 attachment** | spec | _VirtualList port spec_ — `spec/attachments/RPC-002/08-virtuallist-port-spec.md` defines the full target API; RPC-094 implements a strict subset (scrollback-only) |
| **RPC-019** | done | ScrollbackList + PageUp/PageDown + stick-to-bottom — no mouse, no Up/Down lines, no scrollbar |
| **RPC-023** | done | Mouse wheel for BoardView + global native text-selection toggle |
| **RPC-024** | done | Multi-session scrollback state + Shift+←/→ |
| **RPC-028** | done | Mouse wheel + WheelVelocity for ALL popups/dialogs/pickers; the `scroll_viewport.rs::WheelVelocity` primitive is reusable by RPC-094 |
| **RPC-078** | done | TS-side native text-selection while preserving mouse scroll wheel |
| **RPC-091** | done | Chunk rendering parity |
| **RPC-093** | done | Thinking streaming parity |
| **RPC-094** (this card) | _new_ | Closes the AgentView scrollback scroll surface gap |

---

## 4. Out-of-scope future work

These items are deliberately **NOT** in RPC-094. Each gets its own card
when its consumer view is ported:

1. **CheckpointViewer port** — needs VirtualList with `group_by`,
   `SelectionMode::Item`, and Enter callback. Track under a future
   `RPC-XXX: Port CheckpointViewer to ratatui`.
2. **TurnContentModal port** — needs select-mode + Tab navigation;
   tied to the future select-mode work.
3. **FileDiffViewer / ChangedFilesViewer port** — needs lazy mode
   (large diffs) and `group_by`. Track under a future
   `RPC-XXX: Port diff viewers to ratatui`.
4. **Full generic `VirtualList<T>` widget** as specified in
   `spec/attachments/RPC-002/08-virtuallist-port-spec.md` — only
   attempted once we have ≥2 consumers in Rust. Until then, the
   scrollback uses purpose-built `ScrollbackList`.

---

## 5. Test surface RPC-094 must cover

(See `port-spec.md` for full details — this is a high-level enumeration.)

1. Mouse wheel ScrollUp inside scrollback rect drops stick + decrements offset by 1.
2. Mouse wheel ScrollDown inside scrollback rect increments offset + re-enters stick at tail.
3. Mouse wheel velocity ramps 1→5 within 150 ms; resets to 1 after gap.
4. Mouse wheel ScrollUp outside scrollback rect (over header / footer / input) is ignored.
5. Up arrow with MultiLineInput cursor on the first line emits scrollback line-up.
6. Down arrow with MultiLineInput cursor on the last line emits scrollback line-down at the bottom edge.
7. Up arrow with cursor mid-buffer stays inside the input (passthrough).
8. Home key from the scrollback (not consumed by input) jumps offset to 0 + drops stick.
9. End key re-enters stick mode (already partially wired via PageDown — verify it also force-snaps offset).
10. Scrollbar gutter renders when total_visual_rows > viewport_height; thumb position matches `offset / total`.
11. Scrollbar hidden when total fits viewport.
12. Pop-up open absorbs the mouse wheel first (RPC-028 invariant preserved).
