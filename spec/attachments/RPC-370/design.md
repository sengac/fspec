# RPC-370 — Render markdown tables with box-drawing characters in Rust chat view

## Problem

The Rust port chat view (the **AgentView** in the `codelet/fspec-tui` crate) does
**not** render markdown tables the way the TypeScript chat view does.

- **TypeScript** (`src/tui/utils/markdown-table-formatter.ts`): converts a markdown
  pipe-table into a full **Unicode box-drawing grid** with borders, column-width
  alignment, and per-column left/center/right alignment derived from the separator
  row colons. Bold headers via `chalk.bold`.
- **Rust** (`codelet/fspec-tui/src/store/agent_view/markdown_tables.rs`,
  `format_markdown_tables`): only **pads pipe-table cells** to a common width and
  re-emits them as plain ASCII pipe rows. No borders, no alignment from colons, no grid.

The user sees raw-ish padded pipes in the Rust chat view instead of the clean
bordered tables the TS view produces.

## Goal

Port the TS box-drawing renderer into the Rust `format_markdown_tables` so AI
markdown tables render as aligned bordered grids in the Rust chat scrollback,
reaching visual parity with the TS chat view (within the constraints below).

## Reference: TypeScript renderer

`renderParsedTable(table)` in `src/tui/utils/markdown-table-formatter.ts`:

```
┌──────┬──────┐
│ col1 │ col2 │
├──────┼──────┤
│ a    │ bb   │
└──────┴──────┘
```

Border construction (per the TS source):
- Top:    `'┌─' + colWidths.map(w => '─'.repeat(w)).join('─┬─') + '─┐'`
- Header: `'│ ' + headerCells.join(' │ ') + ' │'`
- Sep:    `'├─' + colWidths.map(w => '─'.repeat(w)).join('─┼─') + '─┤'`
- Row:    `'│ ' + cells.join(' │ ') + ' │'`
- Bottom: `'└─' + colWidths.map(w => '─'.repeat(w)).join('─┴─') + '─┘'`

Alignment (`parseAlignment`):
- starts AND ends with `:` → **center**
- ends with `:` → **right**
- starts with `:` → **left**
- otherwise → **left** (default)

Padding (`padText`): pads the cell to the column width on the right (left-align),
left (right-align), or split (center).

Column width = max char/visual width of header + all data cells in that column.

## Current Rust behavior (to replace)

`format_markdown_tables` walks lines, detects a contiguous block where each
non-blank trimmed line starts and ends with `|`, then `push_padded_block` pads
each cell and re-emits `| cell | cell |` rows plus a cleaned `| --- | --- |`
separator. **No box borders, no colon alignment.**

Helpers already present and reusable: `is_table_row`, `is_separator_cell`,
`parse_cells`.

## Target Rust behavior

For each detected pipe-table block:

1. **Validate it is a real table** — the block must contain a separator row
   (a row where every cell matches `^[\s:]*-+[\s:]*$`, i.e. dashes with optional
   colons/spaces). If there is **no separator row**, leave the block unchanged
   (pass-through) — it is a pipe block, not a table.
2. **Parse**:
   - Header = first row's cells.
   - Alignment = parse each separator cell's colons → Left / Center / Right.
   - Data rows = rows after the separator. Pad short rows with empty cells to
     header length; truncate/ignore cells beyond header length.
3. **Compute column widths** = max char count across header + data cells per column.
4. **Render** the five-part box-drawing grid exactly like the TS borders above,
   padding each cell with `pad_text(text, width, alignment)`.
5. Replace the block in the output with the rendered grid.

Non-table text and surrounding prose are preserved exactly (table rendered in place).

### Alignment enum

```rust
enum Align { Left, Center, Right }
```

`parse_alignment(separator_cell) -> Align` mirrors the TS rules. Default is Left.

### pad_text

```rust
fn pad_text(text: &str, width: usize, align: Align) -> String
```

- `pad = width.saturating_sub(text.chars().count())`
- Left:   text + spaces
- Right:  spaces + text
- Center: floor(pad/2) leading, remainder trailing

## Scope boundaries (assumptions — see example map)

- **Bold/ANSI header styling is OUT of scope.** The Rust scrollback wrap path
  (`chunk_wrap.rs::wrap_source` → `text_wrap.rs::wrap_to_width`) renders each chunk
  line as a single-color plain `Span` with no embedded-ANSI parser. Emitting ANSI
  bold codes would render as literal escape garbage. Headers render as plain text
  inside the grid.
- **Code-fence-wrapped tables are OUT of scope.** The TS `looksLikeTable` / code-token
  path depends on the `marked` lexer. The Rust Done-finalization path has no markdown
  lexer; only contiguous pipe-table blocks are converted. (Future RPC.)
- **Column width uses char count** as the visual-width proxy, consistent with
  `text_wrap.rs`. Full East-Asian-width handling is deferred (matches existing
  scrollback behavior).

## Integration (WHO CALLS THIS)

`format_markdown_tables` is **already wired**: invoked from
`chunk_processor.rs::handle_done` (line ~206:
`source.text = format_markdown_tables(&source.text);`) at the `Done` turn boundary.
No new call site is required — replacing the function body changes what the user sees
end-to-end. The rendered multi-line grid flows through `wrap_source` →
`wrap_to_width` and is painted by the `ScrollbackList` widget.

## Files

- **Modify**: `codelet/fspec-tui/src/store/agent_view/markdown_tables.rs`
  (replace pipe-padding output with box-drawing rendering; keep the function under
  the 300-line ceiling — extract a submodule if needed).
- **No change required**: `chunk_processor.rs` (call site already exists).

## Acceptance summary

| # | Behavior |
|---|----------|
| 1 | Simple 2-column table → box-drawing grid, columns aligned |
| 2 | Colon separators → left/center/right per-column alignment |
| 3 | Short data row → blank padded cell, grid stays rectangular |
| 4 | Non-table prose → unchanged byte-for-byte |
| 5 | Table embedded in prose → grid in place, surrounding lines kept |
| 6 | Pipe block with no separator row → unchanged (not a table) |

## Testing notes

Rust unit tests in the `#[cfg(test)] mod tests` block of `markdown_tables.rs`.
Assert on the presence and alignment of box-drawing characters (`┌ ┬ ┐ │ ├ ┼ ┤ └ ┴ ┘ ─`),
equal border-row lengths, and correct padding per alignment. Each Gherkin scenario
gets a test with `// @step` comments matching the feature steps exactly.
