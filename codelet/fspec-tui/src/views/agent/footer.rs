//! SessionFooter — 1-row strip at the bottom of AgentView painting
//! `<cwd> [⎇ <branch>]` on the right side.
//!
//! Feature files:
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc029-agent-structure-alignment.feature
//!
//! RPC-029: structural rewrite to match the TS Ink original
//! (`src/tui/components/SessionFooter.tsx`):
//!
//!   - The LEFT side is now empty (the old RPC-013 hints
//!     `Enter=send  Ctrl+C=interrupt  ESC=back` are no longer painted
//!     — the constant `PLACEHOLDER_FOOTER_HINTS` is kept in
//!     `views/agent.rs` purely for the RPC-013 source-shape invariant).
//!   - The row paints a dark-grey `#333333` background on every cell
//!     and is padded horizontally by 1 column on both sides.
//!   - The right side is split into TWO spans: the cwd in dark-grey
//!     (matching TS `chalk.dim`) and the bracketed branch suffix in
//!     cyan (matching TS `chalk.cyan(formatBranchDisplay(...))`).
//!   - The branch glyph is `⎇` (U+2387 ALTERNATIVE KEY SYMBOL) —
//!     RPC-029 reverses the RPC-018 deliberate divergence (`⌥`,
//!     U+2325 OPTION KEY) to match the canonical TS output.
//!
//! ```text
//!                                              ~/projects/fspec [⎇ main]
//! ```
//!
//! cwd shortening replaces a `$HOME` prefix with `~`. The substitution
//! lives here so the wire shape (`WorkspaceInfo.cwd`) stays portable
//! across hosts whose `$HOME` differs.

use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::WorkspaceInfo;

use super::chrome::{horizontal_pad, line_width};
use super::paint_row_bg;

/// RPC-029: dark-grey (`#333333`) row background painted on every cell
/// of the footer strip.
pub(crate) const FOOTER_BG: Color = Color::Rgb(0x33, 0x33, 0x33);

/// SessionFooter widget. The caller owns the WorkspaceInfo snapshot.
pub struct SessionFooter<'a> {
    pub workspace: Option<&'a WorkspaceInfo>,
}

impl<'a> Widget for SessionFooter<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // RPC-029: dark-grey row background on every cell.
        paint_row_bg(area, buf, FOOTER_BG);
        // RPC-029: horizontal padding of 1 column on both sides.
        let inner = horizontal_pad(area, 1);
        if inner.width == 0 {
            return;
        }
        let right_line = self
            .workspace
            .map(build_right_line)
            .unwrap_or_else(|| Line::from(Vec::<Span<'static>>::new()));
        paint_right_aligned(inner, buf, right_line);
    }
}

/// RPC-029: build the right-aligned line as two styled spans —
/// dark-grey cwd, then cyan `[⎇ branch]` suffix when applicable.
fn build_right_line(workspace: &WorkspaceInfo) -> Line<'static> {
    let cwd = shorten_with_home(&workspace.cwd);
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(cwd, Style::default().fg(Color::DarkGray)));
    if let Some(branch) = workspace.git_branch.as_deref() {
        // RPC-029: glyph reverts to ⎇ (U+2387) — see module doc.
        spans.push(Span::styled(
            format!(" [\u{2387} {branch}]"),
            Style::default().fg(Color::Cyan),
        ));
    }
    Line::from(spans)
}

fn paint_right_aligned(inner: Rect, buf: &mut Buffer, line: Line<'static>) {
    let width = line_width(&line);
    if width == 0 || width as u16 > inner.width {
        return;
    }
    let x = inner.x + (inner.width - width as u16);
    Paragraph::new(line).render(
        Rect {
            x,
            y: inner.y,
            width: width as u16,
            height: 1,
        },
        buf,
    );
}

/// Replace a `$HOME` prefix in `cwd` with `~`. Returns `cwd` unchanged
/// when no `$HOME` is set, the path does not start with it, or the
/// match is mid-segment.
fn shorten_with_home(cwd: &str) -> String {
    let home = home_dir();
    if let Some(home) = home {
        let home_str = home.to_string_lossy();
        if !home_str.is_empty() && cwd.starts_with(home_str.as_ref()) {
            let suffix = &cwd[home_str.len()..];
            if suffix.is_empty() || suffix.starts_with('/') {
                return format!("~{suffix}");
            }
        }
    }
    cwd.to_string()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
    }

    #[test]
    fn shorten_with_home_replaces_home_prefix() {
        std::env::set_var("HOME", "/Users/rquast");
        let s = shorten_with_home("/Users/rquast/projects/fspec");
        assert_eq!(s, "~/projects/fspec");
    }

    #[test]
    fn shorten_with_home_leaves_other_paths_alone() {
        std::env::set_var("HOME", "/Users/rquast");
        let s = shorten_with_home("/tmp/scratch");
        assert_eq!(s, "/tmp/scratch");
    }

    #[test]
    fn build_right_line_uses_alternative_key_glyph() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: Some("main".to_string()),
        };
        let line = build_right_line(&ws);
        let text = line_text(&line);
        assert!(text.contains("[\u{2387} main]"), "must use ⎇ U+2387; got: {text:?}");
        assert!(!text.contains("\u{2325}"), "must NOT use ⌥ U+2325; got: {text:?}");
    }

    #[test]
    fn build_right_line_omits_branch_suffix_when_none() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: None,
        };
        let line = build_right_line(&ws);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "/tmp/scratch");
    }

    #[test]
    fn build_right_line_paints_cwd_dim_and_branch_cyan() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: Some("main".to_string()),
        };
        let line = build_right_line(&ws);
        assert_eq!(line.spans[0].style.fg, Some(Color::DarkGray));
        assert_eq!(line.spans[1].style.fg, Some(Color::Cyan));
    }
}
