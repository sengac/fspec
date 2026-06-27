# AST Research — RPC-363 shared diff-viewer helpers

Goal: enumerate the `pub(super)`/private helpers in `views/changed_files/` that
must be lifted into a shared `views/diff_common/` module.

## diff_render.rs / row.rs helpers (AstGrep `pub(super) fn` + private `fn`)

- `views/changed_files/diff_render.rs:26` `pub(super) fn classify(line: &str) -> DiffLineKind`
- `views/changed_files/diff_render.rs:39` `pub(super) fn diff_line(text: &str) -> Line<'_>`
- `views/changed_files/diff_render.rs:15` `pub(super) enum DiffLineKind` (Added/Removed/Hunk/Context)
- `views/changed_files/row.rs:18` `pub(super) fn status_color(change_type: &str) -> Color`
- `views/changed_files/row.rs:30` `fn truncate_path(path: &str, max_width: usize) -> String` (private)
- `views/changed_files/row.rs:49` `pub(super) fn file_row<'a>(file: &'a ChangedFile, selected: bool, width: usize) -> Line<'a>`

## Pane-scrollbar wrapper (render.rs)

- `views/changed_files/render.rs:163` `fn render_pane_scrollbar(content, buf, list_width, scroll, visible, total)`
  delegates to `crate::components::list_scrollbar::render_list_scrollbar` (already shared, RPC-352).

## Consumers in changed_files

- `render.rs:19` `use super::diff_render::diff_line;`
- `render.rs:20` `use super::row::file_row;`
- `render.rs:122` `file_row(...)` call site (files pane)
- `render.rs:151` `diff_line(l)` call site (diff pane)
- `render.rs:127`/`:156` `render_pane_scrollbar(...)` call sites

## Plan

Move all of the above into `views/diff_common/{mod.rs,diff_render.rs,row.rs}`
as `pub`, carry their `#[cfg(test)]` unit tests, then re-point changed_files
imports to `crate::views::diff_common::{diff_line, file_row, render_pane_scrollbar}`
and delete the private copies. No behavior change.
