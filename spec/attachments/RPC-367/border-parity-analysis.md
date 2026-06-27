# RPC-367 — Pane Border/Divider Parity: Rust TUI vs TypeScript Reference

## Problem Statement

The Rust TUI ports of the **Changed Files** view (`codelet/fspec-tui/src/views/changed_files/`)
and the **Checkpoints** view (`codelet/fspec-tui/src/views/checkpoints/`) omit the
inter-pane borders/dividers that the original TypeScript Ink components draw. This is a
visual-parity regression introduced during the Rust port (RPC-354 / RPC-356 / RPC-363 / RPC-364).

The most visible regression is the **Checkpoints view losing the vertical divider** between
the Checkpoints list pane and the Files list pane. Both views also lost the **heading
underline rule** beneath each pane title.

## Reference: How the TypeScript versions draw borders (Ink)

Both TS views use `<Box borderStyle="single" ...>` with individual edges selectively enabled.
They **never set `borderColor`** — focus is conveyed via the header box's green
`backgroundColor`, not via border colour.

### `FileDiffViewer` / `ChangedFilesViewer`
Vertical stack: Files pane (top) above Diff pane (bottom).

```jsx
// Outer container — bottom rule under the whole viewer
<Box flexDirection="column" flexGrow={1}
     borderStyle="single" borderTop={false} borderLeft={false}
     borderRight={false} borderBottom={true}>

  // File-list pane — bottom rule
  <Box flexDirection="column" flexGrow={1} flexBasis={0}
       borderStyle="single" borderBottom={true} ...>

    // Heading box — bottom rule (underline beneath the "Files" title)
    <Box backgroundColor={focusedPane === 'files' ? 'green' : undefined}
         borderStyle="single" borderBottom={true} ...>
      <Text color={focusedPane === 'files' ? 'black' : 'white'}>Files</Text>
    </Box>
```

Edges actually drawn:
- Outer box: `borderBottom`
- File-list pane box: `borderBottom`
- Each heading box ("Files", "Diff"): `borderBottom`
- Diff pane: **no own border** (it is the last pane)

### `CheckpointViewer`
Three-pane: `[Checkpoints | Files]` row on top, full-width Diff pane below.

```jsx
// Checkpoint-list pane — the ONLY vertical divider in any of these views
<Box flexDirection="column" flexGrow={1} flexBasis={0}
     borderStyle="single" borderRight={true}
     borderTop={false} borderLeft={false} borderBottom={false}>
```

Edges actually drawn:
- Outer box: `borderBottom`
- Top-row box: `borderBottom`
- **Checkpoint-list pane: `borderRight={true}`** ← vertical divider between Checkpoints and Files
- Each heading box: `borderBottom` underline
- File-list pane and Diff pane: no own border (left pane's `borderRight` provides the divider)

## Current state: How the Rust versions draw borders (ratatui)

**None.** Confirmed by reading both `render.rs` files and grepping for
`borders` / `Borders` / `Block::` / `border_style` / `BorderType` — **zero matches** in
`views/changed_files/` and `views/checkpoints/`.

- Panes are separated only by `Layout::split` whitespace gaps.
- Outer chrome comes from `render_full_screen_scaffold_raw_title` (RPC-337): `Clear` + a
  4-row vertical split `[title, blank-separator, body, footer]`. No box-drawing.
- Each pane gets a 1-row `pane_header` painted as a green band when focused
  (`fg(Black).bg(Green).BOLD`) or white text otherwise — but **no underline rule** below it.
- A 1-column scrollbar gutter appears only on overflow.

### Relevant files
| File | Role |
|------|------|
| `codelet/fspec-tui/src/views/changed_files/render.rs` | Dual-pane render; `pane_header`, `render_files_pane`, `render_diff_pane` |
| `codelet/fspec-tui/src/views/checkpoints/render.rs` | Three-pane render; `render_checkpoints_pane`, `render_files_pane`, `render_diff_pane` |
| `codelet/fspec-tui/src/views/diff_common/mod.rs` | Shared helpers (`render_pane_scrollbar`, re-exports). **Best home for a shared divider/header-rule helper.** |
| `codelet/fspec-tui/src/views/full_screen_shell.rs` | Outer scaffold (title/separator/body/footer) |

## The concrete border differences

| Element | TypeScript | Rust (current) | Action |
|---|---|---|---|
| Heading underline | `borderBottom` rule under each pane heading | None (green band only) | Add a `─` rule row beneath each pane header |
| **Checkpoints ↔ Files vertical divider** | **`borderRight` on checkpoint pane** | **None** (whitespace only) | Draw a vertical `│` divider between the two top panes |
| Changed Files: Files ↔ Diff separator | (vertical stack; pane `borderBottom`) | Horizontal split, no divider | Draw a vertical `│` divider between Files and Diff panes |
| Border colour | Never set (default terminal colour) | N/A | Keep default colour (no `borderColor` parity needed) |
| Focus indicator | Green header bg + black/white text | Green header bg + black/white text | Already at parity — **do not change** |

> Note on orientation: TS stacks Files above Diff *vertically* in the Changed Files view,
> whereas Rust uses a *horizontal* 40/60 split. Orientation is a deliberate product choice
> and is **out of scope** for this story — we only restore the divider/underline borders,
> applied to the layout as it currently exists in Rust.

## Proposed implementation approach (DRY)

Add shared helpers to `codelet/fspec-tui/src/views/diff_common/` so both views reuse identical
border rendering (mirroring how RPC-363 already shares `diff_line` / `file_row` /
`render_pane_scrollbar`):

1. A helper that draws a **vertical divider** in a 1-column gutter between two panes using
   `ratatui::widgets::Block::default().borders(Borders::RIGHT)` (or direct buffer cell writes
   with `│`), default colour.
2. A helper (or extension of `pane_header`) that draws a **1-row horizontal underline rule**
   (`─` / `Borders::BOTTOM`) beneath each pane heading.
3. Reserve 1 column / 1 row in the per-pane `Layout` constraints for these dividers so content
   is not overdrawn, and update the cached content `Rect`s used for mouse-wheel hit-testing and
   page-step math accordingly.

### Constraints / guardrails
- **No `borderColor` divergence** — use the default terminal colour, matching TS.
- **Do not change** the focus indicator (green header band) — it is already at parity.
- **Keep files under 300 lines**; both `render.rs` files are already split — prefer the shared
  `diff_common` helper over duplicating divider logic in each view.
- Preserve existing scroll-clamp, scrollbar-gutter, and rect-caching behaviour. The divider
  column must be accounted for so wheel hit-testing (`pane_at`) and `page_step` stay correct.
- No `unwrap()` / `todo!()` / `unimplemented!()` in production paths.

## Acceptance (high level)
1. The Checkpoints view renders a vertical divider between the Checkpoints list pane and the
   Files list pane.
2. The Changed Files view renders a vertical divider between the Files pane and the Diff pane.
3. Both views render a heading underline rule beneath each pane title.
4. Dividers use the default terminal colour (no `borderColor` set), matching the TS reference.
5. Existing behaviour (focus highlight, scrolling, scrollbar gutter, empty-state, mouse-wheel
   hit-testing) is preserved.

## Test strategy
Render each view into a `ratatui::backend::TestBackend` buffer and assert that the expected
box-drawing glyphs (`│` for vertical dividers, `─` for heading underlines) appear at the
expected column/row boundaries between panes — following the existing `TestBackend`
buffer-to-string pattern used in `full_screen_shell.rs` tests and `changed_files/tests.rs`.
