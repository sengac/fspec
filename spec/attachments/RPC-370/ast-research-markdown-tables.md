# AST Research — RPC-370 box-drawing table rendering

## Scope
Rust port chat-view markdown table rendering. Goal: replace the pipe-padding output
of `format_markdown_tables` with full Unicode box-drawing grid rendering (TS parity).

## Rust target module: `codelet/fspec-tui/src/store/agent_view/markdown_tables.rs`

AST pattern: `pub fn $NAME($$$ARGS) -> $RET { $$$BODY }`
- `markdown_tables.rs:17  pub fn format_markdown_tables(input: &str) -> String`  ← ENTRY (modify)

AST pattern: `fn $NAME($$$ARGS) -> $RET { $$$BODY }` (private helpers — reusable)
- `markdown_tables.rs:44  fn is_table_row(line: &str) -> bool`        ← reuse for block detection
- `markdown_tables.rs:49  fn is_separator_cell(cell: &str) -> bool`   ← reuse for separator detection
- `markdown_tables.rs:54  fn parse_cells(line: &str) -> Vec<String>`  ← reuse for cell parsing
- `markdown_tables.rs:61  fn push_padded_block(out, block)`           ← REPLACE with box-drawing renderer

New helpers to add:
- `enum Align { Left, Center, Right }`
- `fn parse_alignment(separator_cell: &str) -> Align`  (colon rules)
- `fn pad_text(text: &str, width: usize, align: Align) -> String`
- `fn render_table_block(out: &mut String, block: &[&str])`  (box-drawing)

## Call site (already wired — no new integration needed)

AST pattern: `format_markdown_tables($$$ARGS)`
- `chunk_processor.rs:206  source.text = format_markdown_tables(&source.text);`
  Invoked inside `handle_done` at the `Done` turn boundary. The rendered grid then
  flows through `chunk_wrap.rs::wrap_source` → `text_wrap.rs::wrap_to_width` → painted
  by `ScrollbackList`. Replacing the function body changes end-user output directly.

## TS reference (parity source)
`src/tui/utils/markdown-table-formatter.ts`:
- `renderParsedTable(table)` — builds ┌─┬─┐ / │ / ├─┼─┤ / └─┴─┘ borders.
- `parseAlignment(separatorCell)` — colon rules (left/center/right/default-left).
- `padText(text, width, alignment)` — left/right/center padding.
- `parseRawTable(text)` — header/separator/data parsing with row padding.

## Constraints
- Keep `markdown_tables.rs` under 300 lines (extract submodule if needed).
- No ANSI bold (scrollback wrap path renders single-color plain spans).
- char-count width proxy (consistent with text_wrap.rs).
