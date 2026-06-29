# RPC-383 — TurnContentModal Parity Analysis

**Type:** Bug
**Epic:** rust-frontend
**Depends on:** RPC-382 (introduced the modal)
**Reference (source of truth):** `src/tui/components/TurnContentModal.tsx`
**Rust implementation under fix:** `codelet/fspec-tui/src/views/agent/turn_modal.rs`

---

## 1. Summary

The Rust port of `TurnContentModal` (RPC-382) diverges from the TypeScript
Ink reference in two user-visible ways:

1. **Sizing bug** — the Rust modal *shrinks to content* (it passes
   `min_width: 0` to `dialog_theme::dialog_rect`, and its height is the
   *natural* row count). The TS reference is a **fixed full-screen
   overlay**: `width = terminalWidth - 4`, `height = terminalHeight - 6`.

2. **Scrolling bug** — the Rust modal has **no scrolling whatsoever**:
   no scroll offset, no scrollbar, no keyboard scroll, no mouse wheel.
   Overflowing body text is **hard-clipped** (`wrapped_rows()` returns
   early once `rows.len() >= max_rows`), silently dropping content. The
   TS reference renders the body in a **scrollable `VirtualList`** with a
   **visible scrollbar** (`showScrollbar={true}`) and a footer hint
   **`↑↓ Scroll | Esc Close`**.

Both bugs are confirmed by reading both implementations (see §2/§3).

---

## 2. TypeScript reference behaviour (`TurnContentModal.tsx`)

| Aspect | Reference behaviour | Source |
|---|---|---|
| Outer box | `position="absolute"`, `width="100%"`, `height="100%"`, centered | lines 131-138 |
| Modal box | `width={terminalWidth - 4}`, `height={terminalHeight - 6}`, rounded cyan border, `padding={1}`, black bg | lines 139-147 |
| Title | Bold cyan, role-derived (`User Message` / `Assistant Response` / `Tool Output` / `Supervisor Input`) | lines 37-50, 149-153 |
| Body | `VirtualList` filling `flexGrow={1}`, wrapped at `terminalWidth - 10` | lines 127-167 |
| Scrollbar | `showScrollbar={true}` | line 162 |
| Scroll mode | `selectionMode="scroll"`, `scrollToEnd={false}`, `isFocused` drives input | lines 163-165 |
| Footer | `↑↓ Scroll | Esc Close` (dim) | lines 170-172 |

Key point: the modal is **always the same size regardless of content
length** (full screen minus margins), and the body is an independently
scrollable viewport.

---

## 3. Rust current behaviour (`turn_modal.rs` + `dialog_theme.rs`)

| Aspect | Current Rust behaviour | Source |
|---|---|---|
| Sizing | **Shrink-to-content**, `min_width: 0` → width = widest wrapped line + 4, clamped to screen | `turn_modal.rs:67-73`, `dialog_theme.rs:103-121` |
| Height | **Natural** = `6 + body_rows`, clamped to screen | `dialog_theme.rs:108-112` |
| Overflow | **Hard-clipped** to `area.height - 6` rows; excess dropped | `turn_modal.rs:79-98` |
| Scrollbar | **None** | n/a |
| Keyboard scroll | **None** — `Up`/`Down` are gated to no-ops while modal open | `dispatch_select.rs:24-37` |
| Mouse wheel | **None** — modal is render-only, no input handler | `mouse_dispatch.rs` never routes to modal |
| Scroll state | **None** — struct holds only `title`, `accent`, `body` | `turn_modal.rs:24-28` |
| Footer | Empty (`footer: ""`) | `turn_modal.rs:71` |

---

## 4. What must change

### 4.1 Full-screen sizing
The modal must paint at a **fixed** rect of `area.width - 4` ×
`area.height - 6` (matching TS `terminalWidth-4` / `terminalHeight-6`),
centered, **independent of content length** — not shrink-to-content.

Implementation options (worker decides, must stay DRY and keep
`dialog_theme` reusable for other dialogs that *do* shrink-to-content):
- Extend `FspecDialog` / `dialog_rect` with an explicit fixed-size /
  "fill" mode, **or**
- Have `TurnContentModal::render` compute and pass a forced size while
  leaving the default shrink-to-content path untouched for other callers.

The existing shrink-to-content behaviour of all *other* dialogs
(confirm, model selector, thinking level, etc.) **must not regress**.

### 4.2 Scrolling
Add a scrollable viewport to the modal body:
- **Scroll state**: a scroll offset for the modal (lives on `AgentView`
  alongside `turn_modal_seq`, or in a dedicated modal state struct — the
  offset must reset to 0 each time the modal opens).
- **Keyboard**: while the modal is open, `Up`/`Down` scroll by one row;
  `PageUp`/`PageDown` scroll by a viewport page; `Home`/`End` jump to
  top/bottom. These currently no-op in `dispatch_select.rs:24-37` and
  must be re-wired to scroll the modal (NOT move the underlying turn
  selection — the gate that protects the selection must remain).
- **Mouse wheel**: route `ScrollUp`/`ScrollDown` to the modal while it is
  open (`mouse_dispatch.rs`), mirroring how the scrollback handles wheel.
- **Scrollbar**: render a single-column scrollbar when content overflows,
  reusing `scrollback_paint::paint_scrollbar` (the canonical `■`/`│`
  DIM-styled painter) — do NOT write a second scrollbar implementation.
- **Windowing**: `wrapped_rows()` must window by the scroll offset (skip
  `offset` rows from the top) instead of clipping from row 0, and must
  clamp the offset so the last page is fully visible.
- **Footer**: render `↑↓ Scroll | Esc Close` (dim, centered) to match TS.

---

## 5. Constraints & guardrails

- **ACDD**: feature scenarios → failing Rust tests (`*_parity_rpcNNN.rs`)
  → implementation. Tests assert parity with the TS reference.
- **File size**: all touched Rust files must stay **< 300 lines**
  (`source_shape` guard tests). Extract helpers if needed.
- **No regressions**: existing RPC-382 scenarios (open/close/Esc
  cascade/Tab tear-down/selection-gating) must still pass. The Up/Down
  *selection* must still NOT move while the modal is open.
- **Reuse**: `paint_scrollbar`, `dialog_theme`, `text_wrap`,
  `wrap_to_width` — no duplicate implementations.
- **Build/quality**: `cargo build -p codelet-fspec-tui`,
  `cargo test -p codelet-fspec-tui`, `cargo clippy -p codelet-fspec-tui`,
  `cargo fmt` all clean. No new clippy warnings.

---

## 6. Acceptance summary (maps to rules)

1. The turn content modal fills the screen (area.width-4 × area.height-6),
   centered, regardless of content length.
2. When the body exceeds the viewport, a scrollbar is shown.
3. Up/Down scroll the modal body by one row while the modal is open
   (without moving the underlying turn selection).
4. PageUp/PageDown scroll by a page; Home/End jump to top/bottom.
5. The mouse wheel scrolls the modal body while it is open.
6. The modal shows a dim footer `↑↓ Scroll | Esc Close`.
7. Opening the modal resets the scroll offset to the top.
8. No content is silently dropped — all body text is reachable by
   scrolling.
9. Existing RPC-382 behaviour (Esc cascade, Tab tear-down, selection
   gating) is preserved.
