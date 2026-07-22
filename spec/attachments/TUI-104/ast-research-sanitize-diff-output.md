# AST Research for TUI-104: Sanitize Diff Output

## Research Summary

### Key Findings

1. **`sanitize_for_terminal()` location**: `store/agent_view/sanitize.rs`
   - Function signature: `pub fn sanitize_for_terminal(text: &str) -> String`
   - Currently only accessible via `store::agent_view::sanitize` module
   - Used in `chunk_processor.rs` via `super::sanitize::sanitize_for_terminal`

2. **`diff_line()` location**: `views/diff_common/diff_render.rs`
   - Function signature: `pub fn diff_line(text: &str) -> Line<'_>`
   - Currently passes raw text directly to `Span::styled(text.to_string(), style)`
   - No sanitization applied

3. **`file_row()` location**: `views/diff_common/row.rs`
   - Function signature: `pub fn file_row(file: &ChangedFile, selected: bool, width: usize) -> Line<'_>`
   - Passes raw `file.path` to `truncate_path()` and then to `Span::styled()`
   - No sanitization applied

4. **`checkpoint_line()` location**: `views/checkpoints/render.rs`
   - Function signature: `fn checkpoint_line(cp: &CheckpointInfo, selected: bool, width: usize) -> Line<'static>`
   - Uses `checkpoint_label(cp)` then `truncate_path()` then `Span::styled()`
   - No sanitization applied

### Module Structure

- `lib.rs` already re-exports `sanitize_for_terminal` (added during this research)
- `views/diff_common/mod.rs` exports `diff_line`, `file_row`, `truncate_path`
- `views/checkpoints/render.rs` imports from `crate::views::diff_common`

### Dependencies

- `sanitize_for_terminal` depends on `regex::Regex` and `std::sync::LazyLock`
- `regex` is already a dependency in `Cargo.toml`
- No new dependencies needed
