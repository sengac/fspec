# RPC-363 — Lift changed_files diff/row/scrollbar helpers into a shared module

## Goal
The CheckpointsView needs the SAME colored diff rendering and file-row formatting as ChangedFilesView.
Rather than duplicate that logic (DRY violation), lift it out of the `changed_files`-private scope into
a shared module both views import.

## Current state
These helpers are currently `pub(super)` inside `views/changed_files/`:
- `views/changed_files/diff_render.rs`:
  - `classify(line: &str) -> DiffLineKind` (26)
  - `diff_line(text: &str) -> Line<'_>` (39)  — colors: hunk dim/cyan, +add green, -remove red
- `views/changed_files/row.rs`:
  - `status_color(change_type: &str) -> Color` (18)
  - `truncate_path(path: &str, max_width: usize) -> String` (30)
  - `file_row(file: &ChangedFile, selected: bool, width: usize) -> Line` (49)
- Already shared: `components/list_scrollbar.rs::render_list_scrollbar` (RPC-352) and
  `views/changed_files/render.rs::render_pane_scrollbar` (the pane-gutter wrapper).

## Required change
1. Create a shared module — recommended `codelet/fspec-tui/src/views/diff_common/` (mod.rs +
   diff_render.rs + row.rs) OR `components/diff_view/` — and MOVE `diff_line`/`classify` and
   `file_row`/`status_color`/`truncate_path` there as `pub` items, carrying their unit tests.
2. Also lift the pane-scrollbar gutter wrapper (`render_pane_scrollbar`) into the shared module so
   both views render scrollbars identically (it already delegates to `list_scrollbar`).
3. Update `views/changed_files/` to import from the shared module. Delete the now-empty private
   copies. NO behavior change.
4. Keep every file < 300 lines.

## Constraints / guardrails
- This is a pure refactor: **all existing `changed_files` tests (currently 18/18 scenarios, ~21 unit
  tests) must remain green with zero edits to their assertions** (other than import paths if a test
  references a moved item directly).
- The moved helpers keep identical signatures and behavior (byte-identical diff colors / row layout).
- `cargo build` + `cargo clippy` clean; no dead code; no `unwrap/expect/panic` added.

## Acceptance criteria
- A shared module exposes `diff_line`, `classify`, `file_row`, `status_color`, `truncate_path`, and a
  pane-scrollbar helper as `pub`.
- `views/changed_files/` consumes the shared module; no duplicated diff/row logic remains.
- Full crate `cargo test` is green (changed_files coverage still 100%, 18/18).
- The shared helpers' own unit tests live with them and pass.

## ACDD note
This is a refactor story. Write tests that pin the shared module's public API (e.g. `diff_line`
classifies +/-/@@ correctly; `file_row` shows the cursor + truncates) BEFORE moving, or move-then-verify
via the relocated tests — either way the feature file scenarios should assert the shared helpers'
observable behavior, and coverage must link to the shared module's implementation.

## Key files
- New: `codelet/fspec-tui/src/views/diff_common/` (or `components/diff_view/`)
- Modified: `views/changed_files/{mod.rs,render.rs,diff_render.rs,row.rs}`, `views/mod.rs`
- Reference: `components/list_scrollbar.rs`
