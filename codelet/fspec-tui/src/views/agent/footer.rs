//! SessionFooter — 1-row strip at the bottom of AgentView painting
//! input hints on the left and `<cwd> [⌥ <branch>]` on the right.
//!
//! Feature: spec/features/rpc018-agent-chrome.feature
//! Card: RPC-018.
//!
//! Mirrors the TS `src/tui/components/SessionFooter.tsx` layout (with
//! the per-RPC-018 architecture-note swap of `⎇` → `⌥`):
//!
//! ```text
//!  Enter=send  Ctrl+C=interrupt  ESC=back      ~/projects/fspec [⌥ main]
//! ```
//!
//! The right side renders:
//!   - just the cwd when `workspace.git_branch` is `None`,
//!   - the cwd plus a `[⌥ <branch>]` suffix when a branch is present,
//!   - nothing at all when `workspace` is `None` (the strip still
//!     paints the hints on the left).
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

/// SessionFooter widget. The caller owns the WorkspaceInfo snapshot.
pub struct SessionFooter<'a> {
    pub workspace: Option<&'a WorkspaceInfo>,
}

impl<'a> Widget for SessionFooter<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let left = build_left_hints();
        let right = self
            .workspace
            .map(build_right_text)
            .unwrap_or_default();
        paint_two_columns(area, buf, &left, &right);
    }
}

fn build_left_hints() -> String {
    super::PLACEHOLDER_FOOTER_HINTS.to_string()
}

fn build_right_text(workspace: &WorkspaceInfo) -> String {
    let mut out = shorten_with_home(&workspace.cwd);
    if let Some(branch) = workspace.git_branch.as_deref() {
        out.push_str(" [⌥ ");
        out.push_str(branch);
        out.push(']');
    }
    out
}

/// Replace a `$HOME` prefix in `cwd` with `~`. Returns `cwd` unchanged
/// when no `$HOME` is set, the path does not start with it, or the
/// match is mid-segment (e.g. `$HOME=/Users/rq` vs `cwd=/Users/rquast/x`).
fn shorten_with_home(cwd: &str) -> String {
    let home = home_dir();
    if let Some(home) = home {
        let home_str = home.to_string_lossy();
        if !home_str.is_empty() && cwd.starts_with(home_str.as_ref()) {
            let suffix = &cwd[home_str.len()..];
            // Only substitute when the home prefix ends at a path
            // boundary — either at the end of the string or right
            // before a path separator. Otherwise `$HOME=/Users/rq`
            // would mangle `/Users/rquast/...` into `~uast/...`.
            if suffix.is_empty() || suffix.starts_with('/') {
                return format!("~{suffix}");
            }
        }
    }
    cwd.to_string()
}

/// Pluck `$HOME` from the environment. We avoid the `home` crate so
/// the workspace doesn't pick up another transitive dep just for this.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn paint_two_columns(area: Rect, buf: &mut Buffer, left: &str, right: &str) {
    let width = area.width as usize;
    let right_len = right.chars().count();
    let budget_left = width.saturating_sub(right_len).saturating_sub(1);
    let left_truncated: String = left.chars().take(budget_left).collect();
    let left_len = left_truncated.chars().count();
    let hint_style = Style::default().fg(Color::DarkGray);
    let path_style = Style::default().fg(Color::Cyan);
    let left_line = Line::from(Span::styled(left_truncated, hint_style));
    Paragraph::new(left_line).render(
        Rect {
            x: area.x,
            y: area.y,
            width: left_len as u16,
            height: 1,
        },
        buf,
    );
    if right_len > 0 {
        let right_x =
            area.x.saturating_add(area.width.saturating_sub(right_len as u16));
        let right_line = Line::from(Span::styled(right.to_string(), path_style));
        Paragraph::new(right_line).render(
            Rect {
                x: right_x,
                y: area.y,
                width: right_len as u16,
                height: 1,
            },
            buf,
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

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
    fn build_right_text_renders_branch_when_present() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: Some("main".to_string()),
        };
        let s = build_right_text(&ws);
        assert!(s.contains("/tmp/scratch"));
        assert!(s.contains("[⌥ main]"));
    }

    #[test]
    fn build_right_text_omits_branch_when_none() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: None,
        };
        let s = build_right_text(&ws);
        assert_eq!(s, "/tmp/scratch");
    }
}
