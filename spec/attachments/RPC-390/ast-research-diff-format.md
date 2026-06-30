# RPC-390 AST Research — Diff Format Port

## Goal
Port TS Edit/Write diff generation into a new pure Rust module
`codelet/fspec-tui/src/store/agent_view/diff_format.rs`. Identify existing
`similar` crate usage to mirror the Myers-line-diff API, and the existing
collapse/tree-connector style in `chunk_wrap.rs`.

## `similar::TextDiff` usage already in the workspace (`codelet/git`)

AstGrep `pattern = "TextDiff::from_lines($$$ARGS)"` (lang rust):

- `codelet/git/src/diff.rs:210` — `TextDiff::from_lines(old_content, new_content)`
- `codelet/git/src/session_result.rs:395` — `TextDiff::from_lines(old_str.as_ref(), new_str.as_ref())`

AstGrep `pattern = "change.tag()"`:

- `codelet/git/src/session_result.rs:400` — iterates `diff.iter_all_changes()`
  and maps `change.tag()`:
  - `ChangeTag::Delete => "-"`
  - `ChangeTag::Insert => "+"`
  - `ChangeTag::Equal  => " "`
  using `change.value().trim_end_matches('\n')`.

This is exactly the shape RPC-390 needs: iterate `iter_all_changes()`, tag →
prefix, strip the trailing newline of each change value to get one source line
per change. Mirrors the TS `changesToDiffLines` `split('\n').filter(len>0)`.

## Collapse / tree-connector style reference (`chunk_wrap.rs`)

`collapse_tool_body` and `wrap_tool_call` in
`codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` already implement:
- `... +N lines (Enter to view full)` collapse indicator (settled),
- threshold-based slicing returning `(Vec, Option<String>)`.

RPC-390's `format_diff_for_display` uses the analogous indicator
`... +N lines (select turn to /expand)` and gap markers `... (N lines)`, plus
`format_with_tree_connectors` (`L `/two-space indent). The module follows the
same `#[cfg(test)]` allow-block convention.

## Decision
Use `similar = "2"` (already in `Cargo.lock` at 2.7.0). Build a minimal
`DiffLine { content, kind }` + `DiffOutputLine { content, kind }` with a
`DiffKind`/`DiffOutputKind { Context, Added, Removed }` enum (no color field —
RPC-391 applies RGB at render time). No `unwrap` in production paths;
`calculate_start_line` uses `std::fs::read_to_string` and returns 1 on any IO
error.
