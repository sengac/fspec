//! RPC-356 / RPC-363 — colored diff-line rendering shared across the
//! Changed Files and Checkpoints diff panes.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Classifies a raw unified-diff line and produces a styled ratatui
//! `Line`: `+` add lines green, `-` remove lines red, `@@` hunk headers
//! (and `--- ` / `+++ ` file headers) dim/cyan, context lines default.
//! Mirrors the TS `diff-parser` + `FileDiffViewer.renderDiffLine`.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// The visual classification of a single diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Hunk,
    Context,
}

/// Classify a raw diff line by its leading character(s).
///
/// File headers (`--- `/`+++ `) are treated as hunk metadata so they do
/// NOT colour as add/remove content (matches the TS parser).
pub fn classify(line: &str) -> DiffLineKind {
    if line.starts_with("@@") || line.starts_with("--- ") || line.starts_with("+++ ") {
        DiffLineKind::Hunk
    } else if line.starts_with('+') {
        DiffLineKind::Added
    } else if line.starts_with('-') {
        DiffLineKind::Removed
    } else {
        DiffLineKind::Context
    }
}

/// Build the styled `Line` for one diff row.
pub fn diff_line(text: &str) -> Line<'_> {
    let style = match classify(text) {
        DiffLineKind::Added => Style::default().fg(Color::Green),
        DiffLineKind::Removed => Style::default().fg(Color::Red),
        DiffLineKind::Hunk => Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        DiffLineKind::Context => Style::default().fg(Color::White),
    };
    Line::from(Span::styled(text.to_string(), style))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn classify_recognizes_each_kind() {
        assert_eq!(classify("+added"), DiffLineKind::Added);
        assert_eq!(classify("-removed"), DiffLineKind::Removed);
        assert_eq!(classify("@@ -1,2 +1,3 @@"), DiffLineKind::Hunk);
        assert_eq!(classify("--- a/file"), DiffLineKind::Hunk);
        assert_eq!(classify("+++ b/file"), DiffLineKind::Hunk);
        assert_eq!(classify(" context"), DiffLineKind::Context);
    }
}
