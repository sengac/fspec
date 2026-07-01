# RPC-393 — Refactor Edit/Write Diff Formatting to a Clean Structured-Row Model

## Summary

The Rust Edit/Write diff pipeline (RPC-390 generation, RPC-391 decode, RPC-392 padding) is a
faithful port of the TypeScript reference — **including the reference's awkward formatting
inconsistencies**. This work unit refactors the pipeline to a clean, logical, efficient
algorithm **while preserving the exact same on-screen formatting** (column layout, colors,
collapse behaviour). Per the user: "do not necessarily copy what TypeScript has — we just
want the same formatting," done with a "clean and efficient algorithm for the highlighting
and indenting."

This is a **pure parity refactor**. No TypeScript changes. No visual regression.

---

## The Defects We Are Fixing (confirmed by deepsearch of the TS reference)

### (A) Gutter color flips by row type — the main visual inconsistency
- Changed `[R]`/`[A]` rows paint the **entire** line — including the line-number gutter —
  white-on-red / white-on-green (gutter is *inside* the colored bar).
- Context rows paint the line-number gutter **gray** with **no** background.
- Result: scanning a diff vertically, the line-number column flips between "gray, no
  background" and "white on solid red/green". Same column, two stylings.

### (C) `[R]`/`[A]` are string steganography, not data — fragile + inefficient
- `diff_format.rs` computes each line's kind, then **encodes it as literal text**:
  `"{line_num} [R]- {content}"`, `"{line_num} [A]+ {content}"`, `"{line_num}   {content}"`.
- `diff_decode.rs` then **re-derives** the kind at render time with `line.find("[R]")`,
  `line.find("[A]")`, and a hand-rolled `context_gutter_len` byte-scanner, then `strip_marker`
  removes the 3-char token, then RPC-392 `pad_to_width` re-pads. The color information was
  computed, thrown away into a string, and reverse-engineered back out — with strip-then-repad
  round-tripping per line, every frame.
- This couples the formatter and the renderer through a brittle string contract (the gutter
  regex `^[L ]?\s*\d+\s{3}`), and is wasteful.

### (D/E) Two divergent "..." styles
- Gap markers `"{gutter-pad} ... (N lines)"` are gutter-indented and fall through to the
  *default* (un-styled) render path.
- Collapse hints `"... +N lines (select turn to /expand)"` have **no** gutter indent.
- Two different left offsets / stylings for conceptually-similar "elision" rows.

---

## Design: Typed Structured Rows End-to-End

Replace the marker-encoded string as the **interchange format** with a typed row model. The
renderer consumes data, not parsed markers.

### New row type (lives in `diff_format.rs`)

```rust
/// A fully-resolved diff display row. The formatter emits these; the
/// renderer styles them directly — NO string markers, NO re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffDisplayRow {
    /// A changed/context source line with its 1-based file line number.
    Removed { line_no: usize, text: String },
    Added   { line_no: usize, text: String },
    Context { line_no: usize, text: String },
    /// An elision row: a context gap ("... (N lines)") or the collapse
    /// hint ("... +N lines (select turn to /expand)"). One uniform kind.
    Elision { text: String },
}
```

`format_diff_for_display` is replaced by a function returning `Vec<DiffDisplayRow>`
(e.g. `build_diff_rows(diff_lines, visible_lines, start_line) -> Vec<DiffDisplayRow>`)
that does the SAME windowing (CONTEXT_LINES=3), gap detection, and collapse logic it does
today — but emits typed rows instead of formatted strings. Keep `DIFF_COLLAPSED_LINES = 25`
and `CONTEXT_LINES = 3`.

### The CRITICAL storage/re-wrap constraint (read carefully)

The diff body is stored as `ChunkSource.text: String` and **re-wrapped on every terminal
resize** by `wrap_source` → `scrollback.rewrap_at` → `wrap_source` again. The modal likewise
re-wraps `full_text: String`. So a structured `Vec<DiffDisplayRow>` cannot simply replace the
stored string unless we either:

- **Option 1 (recommended): serialize rows to a canonical string, parse back to typed rows at
  wrap time.** Keep `ChunkSource.text`/`full_text` as `String` (no struct/store churn), but make
  the serialization an **explicit, single-purpose, unambiguous encoding** owned by ONE module
  with a `to_line(&DiffDisplayRow) -> String` + `parse_line(&str) -> DiffDisplayRow` pair that
  are exact inverses, unit-tested for round-trip. This removes the *ad hoc* `[R]`/`[A]`
  steganography and the regex: the encode/decode become a deliberate, tested, private codec
  rather than two independently-evolving string heuristics. The renderer calls `parse_line`
  then a single `style_row` function. Gutter styling is decided in ONE place.
- **Option 2: carry `Vec<DiffDisplayRow>` on `ChunkSource`** (new field, e.g.
  `diff_rows: Option<Vec<DiffDisplayRow>>`) and width-wrap the typed rows directly. Cleaner in
  theory but touches `ChunkSource`, `session_context.rs`, `chunk_processor.rs`, and every
  `ChunkSource { .. }` literal, and the modal `full_text`. Higher blast radius.

**Pick Option 1 unless you find a compelling reason for Option 2.** Option 1 keeps the store
shape stable (important: `rpc024/026-source-shape.feature` pins these modules under 300 LoC and
the `ChunkSource` shape) while still eliminating the steganography: the encoding becomes a
proper *codec* (one writer, one reader, proven inverse) rather than a formatter writing markers
that a regex in a different module hopes to recognize.

> Whichever option: the diff body MUST still survive width re-wrap (resize) and the modal MUST
> still show the full uncollapsed diff. Existing RPC-391/392 integration tests must stay green.

### Single uniform styling function

One function turns a `DiffDisplayRow` (plus the render width) into the ratatui spans:

```rust
fn style_row(row: &DiffDisplayRow, width: usize) -> Vec<Span<'static>> { … }
```

Rules (this is where we FIX the inconsistencies; choose the consistent option and document it):

1. **Column layout is identical for every row type**: `[line# gutter][marker col][content]`,
   line number right-aligned to a shared width (min 3), exactly as today's visible output.
   The tree-connector (`L `/`  `) handling stays as it is (applied by `format_with_tree_connectors`
   — keep that, both branches are 2 cols so alignment is fine).
2. **Removed/Added**: dark-red `#8B0000` / dark-green `#006400` background, white fg, **padded
   full-width** (preserve RPC-392). Marker glyph is `-` / `+` in the marker column.
3. **Context**: gray line-number gutter + default/white content (NO background) — unchanged.
4. **CONSISTENT GUTTER (fix A)**: decide ONE rule and apply it to ALL row types. RECOMMENDED:
   the line-number gutter is **always** rendered with the same dim/gray style and is **outside**
   the colored background; the colored background fills from the marker column to the right edge.
   This makes the gutter column visually uniform top-to-bottom while keeping the red/green bar.
   (If the worker finds during testing that matching the *current* screenshot exactly requires
   the gutter inside the bar, that is acceptable — but then it must be consistent and the choice
   documented in the feature architecture note. The non-negotiable is: NO per-row-type flip.)
5. **Elision (fix D/E)**: gap markers and collapse hints both render through the SAME helper
   with the SAME indentation and the SAME dim style. One elision style, one offset.

### Renderer wiring

- `chunk_wrap.rs` diff branch: replace `is_decoded_diff_line` + `decode_diff_line_padded` with
  `parse_line` (or direct typed rows) → `style_row(row, width)`.
- `turn_modal.rs` `decode_modal_row`: same — parse/own typed row → `style_row(row, content_width)`.
- Delete the now-dead `[R]`/`[A]` find / `context_gutter_len` / `strip_marker` heuristics (or
  fold them into the single private codec). No dead code left behind.

---

## Acceptance Criteria

1. The diff display pipeline represents each row as a typed `DiffDisplayRow`
   (Removed/Added/Context/Elision) rather than a marker-encoded string used as a color channel.
2. There is exactly ONE function that maps a row + width to styled spans; the scrollback and the
   modal both use it (no duplicated styling logic).
3. The line-number gutter is styled CONSISTENTLY across all row types (no per-row-type flip
   between gray-no-bg and white-on-color). The chosen rule is documented in the feature
   architecture note.
4. Removed lines: `#8B0000` bg + white fg, padded full-width. Added lines: `#006400` bg + white
   fg, padded full-width (RPC-392 preserved).
5. Context lines: gray gutter, white/default content, no background.
6. Gap markers and collapse hints render through ONE shared elision helper with identical
   indentation and dim styling.
7. The visible column layout (line-number width ≥ 3, right-aligned; `-`/`+` aligned with the
   context middle column; content start column) is unchanged from the current output for a given
   diff.
8. The diff body survives a terminal-width re-wrap (resize) — the structured rows are recovered
   correctly after `rewrap_at`, no markers leak to screen, no panic.
9. The turn-content modal shows the FULL uncollapsed diff, styled identically to the scrollback.
10. Non-Edit/Write tool output (Bash/Grep/etc.) is completely unaffected — no diff styling, no
    regression.
11. If Option 1 (string codec) is used: `to_line`/`parse_line` are exact inverses
    (round-trip property test). No remaining `line.find("[R]")` / regex re-derivation outside the
    single codec.
12. No literal `[R]`/`[A]` (or any marker) is ever visible on screen.

---

## Test Plan (every Gherkin step → matching `// @step` comment)

### Unit (in the touched modules)
- **Row construction**: a single-line replacement yields `[…Context, Removed, Added, Context…]`
  with correct `line_no`s and CONTEXT_LINES=3 windowing.
- **Gap rows**: a mid-file change in a 100-line edit yields `Elision` rows for the leading/
  trailing skipped regions (one uniform kind), not bespoke strings.
- **Collapse**: a >25-display-row diff yields the collapse `Elision` hint; the full build does
  not.
- **style_row removed/added**: returns a span set whose total display width == render width, bg
  `Rgb(139,0,0)`/`Rgb(0,100,0)`, fg white, no marker chars; the gutter is styled per the chosen
  consistent rule.
- **style_row context**: gray gutter span + content span, no background, NOT padded full-width.
- **style_row elision**: dim, single uniform indentation; identical for gap vs collapse hint.
- **Gutter consistency property**: the gutter style of a Context row and the gutter region of a
  Removed/Added row follow the SAME rule (assert no per-type flip).
- **Codec round-trip** (if Option 1): `parse_line(to_line(row)) == row` for every variant,
  including unusual content (spaces, `[`, digits, empty text).
- **Zero/small width**: no panic; saturating pad.

### Integration (drive real store + render, like `edit_diff_padding_rpc392.rs`)
- **Scrollback**: push an Edit ToolCall + matching ToolResult; wrap at width 50; assert
  removed/added `Line`s are full-width colored, context line has gray gutter + no bg, gutter
  styling consistent, and NO `[R]`/`[A]` text appears in any line.
- **Resize re-wrap**: wrap at width 50, then `rewrap_at` at width 80; assert the diff still
  renders correctly (rows recovered, colors intact, no marker leakage).
- **Modal**: build `TurnContentModal` over the full diff; render to a `TestBackend`; assert diff
  rows are full-width bars, plain rows are not, gutter consistent, no markers on screen.
- **No-regression**: a Bash tool result renders plain (no diff bg) — unchanged.

---

## Files In Scope

- `codelet/fspec-tui/src/store/agent_view/diff_format.rs` — typed rows + build function + (Option 1) codec
- `codelet/fspec-tui/src/store/agent_view/diff_decode.rs` — single `style_row`; remove marker heuristics
- `codelet/fspec-tui/src/store/agent_view/pending_tool_diff.rs` — produce via typed rows
- `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` — diff branch uses `style_row`
- `codelet/fspec-tui/src/views/agent/turn_modal.rs` — `decode_modal_row` uses `style_row`
- (Option 2 only) `rendered_chunk.rs`, `chunk_processor.rs`, `session_context.rs`
- Feature: `spec/features/agentview-edit-diff-structured-rows.feature` (new, `@RPC-393`)
- Tests: `#[cfg(test)]` in touched modules + integration test under `codelet/fspec-tui/tests/`

## Out of Scope
- Diff GENERATION algorithm (Myers / `format_edit_diff` / `format_write_diff`) — keep as-is.
- Context-window collapse counts (25 / 8) — keep as-is.
- Any TypeScript code.

## Constraints / Standards
- Rust: no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` in production paths; saturating
  arithmetic; every touched file < 300 LoC; `cargo clippy -p codelet-fspec-tui --all-targets`
  zero warnings; `cargo fmt --check` clean.
- DRY: ONE styling function, ONE elision helper, ONE codec (if used). Reuse the existing
  `chars().count()` display-width proxy (`wrap_to_width`).
- 100% scenario coverage with accurate `link-coverage` line ranges and exact `@step` comments.
- Keep ALL existing RPC-390/391/392 tests green (or migrate them deliberately if the row model
  supersedes a marker-level test — document any such migration).
