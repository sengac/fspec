# AST Research — COPY-009: Wire selection + copy into the BoardView details strip

All paths under `codelet/fspec-tui/`. Package `codelet-fspec-tui`.

## Board module map (line counts — 300-LoC ceiling enforced by source_shape tests)
- `src/views/board.rs` — 230 lines. `struct BoardView` + `handle_event` (key routing). **Tight**: adding fields + Esc handling risks >300; may need to split key-handling into a sibling `#[path]` mod.
- `src/views/board/render.rs` — 122. `render_with_store` caches areas from `split`.
- `src/views/board/mouse.rs` — 101. `handle_mouse` (wheel + click).
- `src/views/board/details_strip.rs` — 267. `render` + text helpers. **Tight** — highlight overlay should live elsewhere (render_with_store or a new sibling), not here.
- `src/views/board/borders.rs` — 38. `paint_side_borders`, `inner_rect`.
- Other: grid.rs 158, viewport.rs 160, columns.rs 67, header.rs 89, footer.rs 41.

## Wiring points

### 1. State (`board.rs:54-84`)
`BoardView` holds NO work-unit state — only `theme`, `action_tx`, and `Cell<..>` render-observed areas:
- `last_viewport_height: Cell<u16>`
- `last_content_area: Cell<Option<Rect>>` (split[7], column content)
- `last_column_header_areas: Cell<Option<[Rect;7]>>`
- `last_column_content_areas: Cell<Option<[Rect;7]>>`
- `emit(&self, action)` at `:93` sends on `action_tx`.

ADD (interior mutability, matching the `Cell` convention):
- `last_details_area: Cell<Option<Rect>>` — set in render to `borders::inner_rect(split[3])`.
- `recognizer` (COPY-003) + `selection: RefCell<Option<Selection>>` (COPY-002) + `Osc52Clipboard` (COPY-001) writer. Follow the AgentView COPY-006/007/008 wiring (recognizer + injected `Vec<u8>` writer via a test seam) — mirror `set_clipboard_writer_for_test` used in the AgentView tests.
- Initialize all in `BoardView::new` (`:75-84`).

### 2. Render caches details rect (`render.rs:76-80`)
```
borders::paint_side_borders(split[3], buf, border_style);
details_strip::render(borders::inner_rect(split[3]), buf, store.selected_work_unit());
```
`last_content_area.set(Some(split[7]))` is at `render.rs:104` — add `view.last_details_area.set(Some(borders::inner_rect(split[3])))` right after the details_strip render call. Paint the REVERSED highlight overlay (COPY-005) over selected strip rows AFTER details_strip::render, in render_with_store (keeps details_strip.rs <300).

### 3. Mouse routing (`mouse.rs:40-95`)
`handle_mouse(view, event, store)` currently:
- wheel inside `last_content_area` → SelectPrev/Next, FocusPrev/NextColumn (`:35-36`)
- `Down(Left)` in header area → `SetFocusedColumn(idx)` (`:81`)
- `Down(Left)` in content cell → `SetFocusedColumn(idx)` + `SelectIndexInFocused(target)` (`:94-95`)

CHANGE: hit-test `view.last_details_area` FIRST. If the event lands inside, convert `(col,row)` → strip-local `(row,col)` by subtracting the rect origin, feed to the SelectionRecognizer (COPY-003), and handle Begin/Extend/Commit (Commit → reconstruct visible strip text, write via OSC 52, retain highlight). Otherwise fall through to existing wheel/click logic UNCHANGED (rule [6]/[7]: click/wheel outside strip is untouched).

### 4. Text reconstruction (no scrollbar — simpler than COPY-004)
`details_strip::render` (`details_strip.rs:31`) produces fixed rows for the selected `WorkUnitInfo`:
- id:title row via `truncate_to(title_text, area.width)` (`:44`, helper `:203`)
- description via `wrap_to_two_lines(&normalized, avail)` → `(line1,line2)` (`:68`, helper `:158`)
- attachments/metadata lines below.
Reproduce the EXACT on-screen rows for the selected row-span, excluding the two vertical border columns painted by `borders::paint_side_borders` (rule [3]/[4]). Fixed 5 rows, no scroll offset.

### 5. Clear seams
- Selection clears when selected work unit changes: actions `SetFocusedColumn` / `SelectIndexInFocused` (emitted at `mouse.rs:81,94,95`) that change `store.selected_work_unit()`; also key-driven `SelectNext/Prev`, `FocusPrev/NextColumn` (`board.rs:148-159`).
- Esc: `board.rs handle_event` key match (`:129+`) has no Esc arm yet — ADD an Esc arm that clears `selection` (no copy on Esc, rule [7]/example [4]/[5]). Because BoardView is state-light, the clear-on-selection-change likely belongs in the mouse/dispatch path where the recognizer lives, or gated on `selected_work_unit()` identity captured at selection start.

## Testing harness (mirror COPY-006/007/008)
Real `BoardView` + `BoardStore` + injected `Vec<u8>` Osc52 writer. Drive real `Event::Mouse` Down→Drag→Up inside the cached details rect; assert exact border-free visible strip bytes via `osc52(...)`. Down OUTSIDE the rect still yields `SetFocusedColumn`/`SelectIndexInFocused`. Separate tests: changing selected work unit clears selection; Esc clears without copy; quick click = no selection/copy; render buffer shows REVERSED cells over selected strip rows, never over the `│` border columns. Prefer real objects over mocks.

## Risks / deviations to watch
- `board.rs` (230) and `details_strip.rs` (267) are both near the 300 ceiling — put new logic in a sibling `#[path=...] mod board/details_select.rs` following the AgentView `turn_modal_select.rs` precedent.
- Reuse COPY-001/002/003/004/005 UNCHANGED.
