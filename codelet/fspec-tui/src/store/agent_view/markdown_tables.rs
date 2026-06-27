//! Public entry point for markdown pipe-table rendering (RPC-370).
//!
//! Feature: spec/features/markdown-table-box-drawing-rendering-in-rust-chat-view.feature
//!
//! Scans accumulated assistant text at `Done` finalisation and renders every
//! contiguous markdown pipe-table as a Unicode box-drawing grid. Grid
//! construction lives in `markdown_table_render.rs`. Pipe blocks without a
//! dash separator row are passed through unchanged.

use super::markdown_table_render::{is_table_row, push_table_block};

/// Detect every contiguous pipe-table block in `input` and render it as a
/// Unicode box-drawing grid. Non-table lines — and pipe blocks without a
/// dash separator row — pass through unchanged.
pub fn format_markdown_tables(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let lines: Vec<&str> = input.split('\n').collect();

    let mut i = 0;
    while i < lines.len() {
        if is_table_row(lines[i]) {
            // Collect the contiguous block of pipe rows.
            let start = i;
            while i < lines.len() && is_table_row(lines[i]) {
                i += 1;
            }
            let block = &lines[start..i];
            push_table_block(&mut out, block);
        } else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
    }
    // Strip trailing newline if the original didn't have one.
    if !input.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

#[cfg(test)]
#[path = "markdown_tables_tests.rs"]
mod tests;
