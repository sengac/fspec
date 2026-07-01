//! RPC-400 — stderr sentinel marker helpers (TypeScript parity).
//!
//! Feature: spec/features/tool-card-stderr-line-coloring.feature
//!
//! Single source of truth in fspec-tui for the `⚠stderr⚠` marker used to
//! flag stderr lines for red styling. The value is locked to codelet-tools
//! `bash_output.rs::STDERR_MARKER` by parity (no cross-crate dependency).
//!
//! Two-stage design mirroring `src/tui/components/AgentView.tsx`:
//!   * live path (`:2485-2490`) — [`mark_stderr_chunk`] converts an
//!     `is_stderr=true` progress chunk into the same in-band marker the
//!     settle path already carries.
//!   * render path (`:5393-5422`) — [`strip_marker`] removes every marker
//!     occurrence so it never reaches the screen.

/// Marker prefixed to stderr lines to enable red styling in the UI.
/// Locked to codelet-tools `bash_output.rs::STDERR_MARKER`.
pub const STDERR_MARKER: &str = "⚠stderr⚠";

/// Prefix each non-empty `\n`-split line of `chunk` with [`STDERR_MARKER`]
/// and re-join with `\n`. Empty lines are left unprefixed (parity with
/// `AgentView.tsx:2485-2490`).
pub fn mark_stderr_chunk(chunk: &str) -> String {
    chunk
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{STDERR_MARKER}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove ALL occurrences of [`STDERR_MARKER`] from `line`.
pub fn strip_marker(line: &str) -> String {
    line.replace(STDERR_MARKER, "")
}

/// Apply [`mark_stderr_chunk`] when `is_stderr`, else return `chunk`
/// unchanged. Keeps the live-fold call site to a single expression.
pub fn maybe_mark(chunk: &str, is_stderr: bool) -> String {
    if is_stderr {
        mark_stderr_chunk(chunk)
    } else {
        chunk.to_string()
    }
}

/// **RPC-400**: style ONE non-diff modal hard line. A line carrying the
/// stderr marker is stripped and styled red (parity with the scrollback);
/// any other line is returned as raw wrapped fragments. `wrap` produces the
/// width-fit fragments (injected so this module stays render-agnostic).
pub fn style_modal_raw_line<F>(line: &str, wrap: F) -> Vec<Vec<ratatui::text::Span<'static>>>
where
    F: FnOnce(&str) -> Vec<String>,
{
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;

    let red = line.contains(STDERR_MARKER);
    let mut frags = wrap(&strip_marker(line));
    if frags.is_empty() {
        frags.push(String::new());
    }
    let style = Style::default().fg(Color::Red);
    frags
        .into_iter()
        .map(|f| {
            vec![if red {
                Span::styled(f, style)
            } else {
                Span::raw(f)
            }]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn marker_value_is_exact() {
        assert_eq!(STDERR_MARKER, "⚠stderr⚠");
    }

    #[test]
    fn mark_prefixes_each_non_empty_line() {
        assert_eq!(
            mark_stderr_chunk("error: boom\nmore"),
            "⚠stderr⚠error: boom\n⚠stderr⚠more"
        );
    }

    #[test]
    fn mark_leaves_empty_lines_unprefixed() {
        assert_eq!(mark_stderr_chunk("a\n\nb"), "⚠stderr⚠a\n\n⚠stderr⚠b");
    }

    #[test]
    fn strip_removes_all_occurrences() {
        assert_eq!(strip_marker("⚠stderr⚠⚠stderr⚠x"), "x");
    }
}
