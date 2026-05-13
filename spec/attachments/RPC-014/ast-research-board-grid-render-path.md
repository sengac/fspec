# RPC-014 — AST research: BoardView render path and WorkUnitInfo

## Goal

Confirm the integration surfaces touched by the rich box-drawing grid +
work-unit details strip port:

  1. Where is `WorkUnitInfo` defined and how many constructor sites need
     updating when `attachments: Vec<String>` is added?
  2. Where does the current BoardView render path live, and what
     function shape will the new `views/board/grid.rs` and
     `views/board/details_strip.rs` modules need to plug into?

## 1. `WorkUnitInfo` definition

```
pattern: pub struct WorkUnitInfo { $$$FIELDS }
language: rust
```

Match (`codelet/rpc-types/src/lib.rs:37`):

```rust
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    #[cfg_attr(feature = "napi", napi(js_name = "workType"))]
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
}
```

The new field `pub attachments: Vec<String>` is added at the bottom, gated
on the `napi` feature attribute alongside the existing `work_type` rename
so the JS shape `attachments: string[]` is preserved on the NAPI boundary.

## 2. Existing `WorkUnitInfo { ... }` literal constructors

Searched with `Grep pattern='WorkUnitInfo \{'` (Grep tool, not bash). All
sites must add the new field; the helpers below default to `Vec::new()`
and only the new RPC-014 render-test fixtures pass a non-empty Vec.

| File | Lines | Default? |
|---|---|---|
| `codelet/core/src/work_units.rs` | 75 | Yes — `From<WorkUnitRecord>` impl |
| `codelet/rpc/src/lib.rs` | 375, 384 | Yes — `test_fixture()` returning two units |
| `codelet/fspec-tui/tests/app_bootstrap_rpc009.rs` | 36 | Yes — `wu()` helper |
| `codelet/fspec-tui/tests/board_agent_navigation_rpc012.rs` | 18 | Yes — `wu()` helper |
| `codelet/fspec-tui/tests/store_board_unit_rpc012.rs` | 13 | Yes — `wu()` helper |
| `codelet/fspec-tui/tests/app_with_mock_backend_repl.rs` | 36 | Yes — `wu()` helper |
| `codelet/fspec-tui/tests/view_board_unit_rpc012.rs` | 19 | Yes — `wu()` helper |
| `codelet/fspec-tui/tests/view_board_unit_rpc013.rs` | 16 | Yes — `wu()` helper |
| `codelet/fspec-tui/src/views/navigator.rs` | 128 | Yes — inline test helper |

Source-shape regression tests that reference the identifier string only
(`rpc-006-source-shape`, `architecture_invariants`) need no change.

## 3. Existing BoardView render path

`codelet/fspec-tui/src/views/board.rs` (≈ 240 lines after RPC-013) holds:

  - `pub fn render_with_store(&self, area, buf, &BoardStore)` — the
    orchestrator. Currently:
      * draws `Block::default().borders(Borders::ALL)`,
      * splits inner into `[Min(0), Length(1)]` for content + footer,
      * uses `Layout::horizontal` to split content evenly into seven
        `Constraint::Length(col_width)` columns,
      * iterates `render_column` per column,
      * paints the RPC-013 footer string on the bottom row.
  - `fn render_column(&self, column, col_idx, area, buf, &BoardStore)` —
    paints uppercase header + column cells, but as a plain `Paragraph`
    inside an unbordered area (no box-drawing junctions).
  - `fn render_footer(area, buf, theme)` — RPC-013 literal footer.

## 4. Target render path after RPC-014

The orchestrator stays in `views/board.rs` but is refactored to compose:

  1. Top border row `┌─...─┐` (full width).
  2. 5-row details strip rendered by `details_strip.rs::render(...)`,
     reading `store.selected_work_unit()`.
  3. Header→content separator row `├┬...┬┤` (top junctions) using
     `grid.rs::build_border_row(widths, "├", "┬", "┤", SeparatorType::Top)`.
  4. Column header row painted cell-by-cell with focused-column cyan
     highlight (`Span::styled(name, cyan|bold)`).
  5. Column-header→content separator row `├┼...┼┤` (cross junctions).
  6. N content rows (`area.height - fixed_rows` per
     `calculate_viewport_height`). Each row is `│ cell │ ... │ cell │`.
     Cells reuse the work-type colors and the selected-cell highlight
     from the current `render_column` but emit via styled `Span`s rather
     than a single `Paragraph::new` per column.
  7. Footer separator row `├┴...┴┤` (bottom junctions).
  8. RPC-013 literal footer string row.
  9. Bottom border row `└─...─┘`.

The orchestrator never breaches 300 LoC because all per-row painting is
delegated to the two helper modules (`grid.rs` for separator strings +
column widths; `details_strip.rs` for the 5-row strip).

## 5. Pure-function `grid.rs` surface

```rust
pub struct ColumnWidths {
    pub base_width: u16,
    pub remainder: u16,
}

pub enum SeparatorType {
    Plain,   // ─
    Top,     // ┬
    Cross,   // ┼
    Bottom,  // ┴
}

pub fn calculate_column_widths(terminal_width: u16) -> ColumnWidths;
pub fn column_width_at(idx: usize, widths: ColumnWidths) -> u16;
pub fn build_border_row(
    widths: ColumnWidths,
    left: &str,
    edge: &str,
    right: &str,
    separator: SeparatorType,
) -> String;
pub fn calculate_viewport_height(terminal_height: u16) -> u16;
```

All four functions are pure (no `ratatui::Buffer` parameter) so they can
be unit-tested directly without spinning up a `TestBackend`.

## 6. Details-strip `details_strip.rs` surface

```rust
pub fn render(
    area: Rect,
    buf: &mut Buffer,
    selected: Option<&WorkUnitInfo>,
    terminal_width: u16,
);
```

Internally lays out the 5 rows by hand: title row, description row
(truncated to `width - 4`), attachments row (basename comma-join with
the "A" key hint), epic+estimate+status row, padding row. When
`selected = None`, paints `No work unit selected` centred on the strip.

## 7. RPC-013 invariants that must NOT regress

  - `navigator.rs` MUST NOT re-introduce `Constraint::Length(1)` for a
    shared footer (each view paints its own).
  - `views/mod.rs` and `lib.rs` MUST NOT re-export `FooterView`.
  - `codelet/fspec-tui/src/views/footer.rs` MUST stay absent.
  - The literal substrings `← → Columns`, `↑↓ Work Units`,
    `[ Priority Up`, `] Priority Down`, `↵ Work Agent`, `ESC Back` MUST
    stay in `board.rs`.

## 8. Tag invariants on `codelet/fspec-tui` Cargo.toml

`source_shape_cargo` and `source_shape_rpc009` already pin the allowed
dependency set. RPC-014 adds NO new crate dependencies — `grid.rs` and
`details_strip.rs` use only `ratatui::{buffer::Buffer, layout::Rect,
style::{Color, Modifier, Style}, text::Span, widgets::Widget}`, plus
`codelet_rpc_types::WorkUnitInfo`. Both are already in the allowed list.
