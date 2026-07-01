# RPC-393 AST Research — Edit/Write Diff Pipeline

AST analysis of the modules in scope, capturing the current public surface that
the structured-row refactor must preserve or deliberately supersede.

## diff_format.rs (generation + current marker-encoded display string)
Public functions (ast pattern `pub fn $NAME(...) -> $RET { ... }`):
- `format_edit_diff(old_string, new_string) -> Vec<DiffOutputLine>` (45) — KEEP (generation, out of scope)
- `format_write_diff(content) -> Vec<DiffOutputLine>` (71) — KEEP (generation)
- `format_with_tree_connectors(content) -> String` (84) — KEEP (tree connector `L `/`  `)
- `format_diff_for_display(diff_lines, visible_lines, start_line) -> String` (105) — SUPERSEDE:
  emits the `[R]`/`[A]` marker steganography. Replaced by `build_diff_rows -> Vec<DiffDisplayRow>`
  plus a single private codec (`to_line`/`parse_line`). A thin string wrapper kept so the
  RPC-390 byte-for-byte golden tests still pass (codec is the canonical string).
- `calculate_start_line(...) -> usize` (247) — KEEP (start-line resolution)

## diff_decode.rs (marker re-parse → spans) — to be replaced by typed style_row
Public functions:
- `decode_diff_line(line) -> Vec<Span>` (34) — SUPERSEDE
- `decode_diff_line_padded(line, width) -> Vec<Span>` (47) — SUPERSEDE by `style_row`
- `is_decoded_diff_line(line) -> bool` (87) — SUPERSEDE (no longer probe markers; we own rows)
- `decode_modal_row(row, width) -> Vec<Span>` (97) — REWIRE to `parse_line` + `style_row`
Private heuristics to DELETE: `context_gutter_len`, `strip_marker`, `colored_span`, `pad_to_width`
(fold padding into the single styling path).

## Call sites (who calls this)
- chunk_wrap.rs diff branch (~136-147): `is_decoded_diff_line` + `decode_diff_line_padded` →
  replace with `parse_line(row)` → `style_row(&row, width)`.
- turn_modal.rs `render` (~116): `decode_modal_row` → `parse_line` + `style_row`.
- pending_tool_diff.rs `produce_diff_strings`: builds `(collapsed, full)` String via
  `format_diff_for_display` → switch to row builder + codec `to_line` join.
- chunk_processor.rs `handle_tool_result` (~158): stores `(collapsed, full)` strings on
  `ChunkSource.text` / `full_text`. UNCHANGED shape (Option 1 keeps String).

## Decision
Option 1 (string codec). The codec `to_line`/`parse_line` becomes the SINGLE writer/reader,
proven inverse by a round-trip property test; the renderer parses then calls ONE `style_row`.
Gutter consistency: gutter always dim/gray OUTSIDE the colored bar (bar fills marker col → edge).
