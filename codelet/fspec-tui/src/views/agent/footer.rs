//! SessionFooter — 1-row strip painting `<cwd> [⎇ <branch>]` on the
//! right side, and (RPC-047) an optional `[compacting: <phase> <c>/<t>]
//! ▰▰▰▰▰▱▱▱▱▱` chip on the left when a compaction is in flight for the
//! focused session.
//!
//! Feature files:
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc029-agent-structure-alignment.feature
//!   - spec/features/slash-command-compact.feature (RPC-047)
//!
//! RPC-029: structural rewrite matches the TS Ink original
//! (`src/tui/components/SessionFooter.tsx`). Row paints a dark-grey
//! `#333333` background on every cell and is padded horizontally by
//! 1 column on both sides. The right side is split into TWO spans:
//! the cwd in dark-grey (matching TS `chalk.dim`) and the bracketed
//! branch suffix in cyan. The branch glyph is `⎇` (U+2387). cwd
//! shortening replaces a `$HOME` prefix with `~`.

use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::{CompactionProgress, WorkspaceInfo};

use super::chrome::{horizontal_pad, line_width};
use super::paint_row_bg;

/// RPC-029: dark-grey (`#333333`) row background on every footer cell.
pub(crate) const FOOTER_BG: Color = Color::Rgb(0x33, 0x33, 0x33);

/// RPC-047: fixed bar width (in cells). Mirrors the 10-cell TS bar.
const COMPACTION_BAR_WIDTH: u16 = 10;

/// SessionFooter widget. Caller owns the `WorkspaceInfo` snapshot AND
/// (RPC-047) the optional `CompactionProgress` for the focused session.
pub struct SessionFooter<'a> {
    pub workspace: Option<&'a WorkspaceInfo>,
    /// When `Some`, the left side paints `[compacting: <phase> <c>/<t>]`
    /// + a 10-cell `▰▰▰▰▰▱▱▱▱▱` bar. When `None`, only the right-aligned
    ///   `cwd [⎇ branch]` is painted (RPC-029 behaviour).
    pub compaction_progress: Option<&'a CompactionProgress>,
    /// RPC-061: per-session pending-supervisor count. When > 0, the
    /// SessionFooter paints `[N pending from supervisor]` (yellow) in
    /// the left-aligned slot, suppressing the compaction chip for that
    /// frame (the supervisor signal is more urgent).
    pub supervisor_pending_count: usize,
}

impl<'a> Widget for SessionFooter<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        paint_row_bg(area, buf, FOOTER_BG);
        let inner = horizontal_pad(area, 1);
        if inner.width == 0 {
            return;
        }
        if self.supervisor_pending_count > 0 {
            // RPC-061: supervisor chip wins over the compaction chip.
            paint_left_aligned(
                inner,
                buf,
                build_left_supervisor_line(self.supervisor_pending_count),
            );
        } else if let Some(progress) = self.compaction_progress {
            paint_left_aligned(inner, buf, build_left_compaction_line(progress));
        }
        let right_line = self
            .workspace
            .map(build_right_line)
            .unwrap_or_else(|| Line::from(Vec::<Span<'static>>::new()));
        paint_right_aligned(inner, buf, right_line);
    }
}

/// RPC-061: build the yellow `[N pending from supervisor]` chip.
fn build_left_supervisor_line(count: usize) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("[{count} pending from supervisor]"),
        Style::default().fg(Color::Yellow),
    )])
}

/// RPC-029: build the right-aligned line as two styled spans —
/// dark-grey cwd, then cyan `[⎇ branch]` suffix when applicable.
fn build_right_line(workspace: &WorkspaceInfo) -> Line<'static> {
    let cwd = shorten_with_home(&workspace.cwd);
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(cwd, Style::default().fg(Color::DarkGray)));
    if let Some(branch) = workspace.git_branch.as_deref() {
        spans.push(Span::styled(
            format!(" [\u{2387} {branch}]"),
            Style::default().fg(Color::Cyan),
        ));
    }
    Line::from(spans)
}

/// RPC-047: build the left-aligned compaction chip + bar.
fn build_left_compaction_line(progress: &CompactionProgress) -> Line<'static> {
    let chip = format!(
        "[compacting: {phase} {current}/{total}]",
        phase = progress.phase,
        current = progress.current,
        total = progress.total,
    );
    let bar = compaction_bar(
        progress.current,
        progress.total,
        COMPACTION_BAR_WIDTH as u32,
    );
    Line::from(vec![
        Span::styled(chip, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(bar, Style::default().fg(Color::Cyan)),
    ])
}

/// RPC-047: fixed-width `▰▰▰▱▱▱` bar with `width` cells. `filled =
/// round(current / total * width)`, saturating to `0..=width`.
/// `total == 0` yields an all-empty bar.
pub(crate) fn compaction_bar(current: u32, total: u32, width: u32) -> String {
    let filled = if total == 0 {
        0
    } else {
        let ratio = (current as f64) / (total as f64);
        (ratio * width as f64).round().clamp(0.0, width as f64) as u32
    };
    let mut out = String::with_capacity(width as usize * 3);
    for i in 0..width {
        if i < filled {
            out.push('\u{25B0}');
        } else {
            out.push('\u{25B1}');
        }
    }
    out
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

/// RPC-047: paint `line` left-aligned, truncating to inner width.
fn paint_left_aligned(inner: Rect, buf: &mut Buffer, line: Line<'static>) {
    let width = line_width(&line);
    if width == 0 {
        return;
    }
    let paint_width = (width as u16).min(inner.width);
    Paragraph::new(line).render(
        Rect {
            x: inner.x,
            y: inner.y,
            width: paint_width,
            height: 1,
        },
        buf,
    );
}

/// Replace a `$HOME` prefix in `cwd` with `~`.
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
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn shorten_with_home_replaces_home_prefix() {
        std::env::set_var("HOME", "/Users/rquast");
        assert_eq!(
            shorten_with_home("/Users/rquast/projects/fspec"),
            "~/projects/fspec"
        );
    }

    #[test]
    fn shorten_with_home_leaves_other_paths_alone() {
        std::env::set_var("HOME", "/Users/rquast");
        assert_eq!(shorten_with_home("/tmp/scratch"), "/tmp/scratch");
    }

    #[test]
    fn build_right_line_uses_alternative_key_glyph() {
        let ws = WorkspaceInfo {
            cwd: "/tmp/scratch".to_string(),
            git_branch: Some("main".to_string()),
        };
        let text = line_text(&build_right_line(&ws));
        assert!(text.contains("[\u{2387} main]"));
        assert!(!text.contains("\u{2325}"));
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

    #[test]
    fn compaction_bar_renders_five_filled_for_5_of_10() {
        let bar = compaction_bar(5, 10, 10);
        assert_eq!(
            bar,
            "\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B1}\u{25B1}\u{25B1}\u{25B1}\u{25B1}"
        );
    }

    #[test]
    fn compaction_bar_renders_all_empty_when_total_is_zero() {
        assert_eq!(compaction_bar(0, 0, 10), "\u{25B1}".repeat(10));
    }

    #[test]
    fn compaction_bar_saturates_when_current_exceeds_total() {
        assert_eq!(compaction_bar(99, 10, 10), "\u{25B0}".repeat(10));
    }

    #[test]
    fn build_left_compaction_line_contains_chip_and_bar() {
        let progress = CompactionProgress {
            phase: "summarising messages".to_string(),
            current: 5,
            total: 10,
        };
        let text = line_text(&build_left_compaction_line(&progress));
        assert!(text.contains("[compacting: summarising messages 5/10]"));
        assert!(text.contains(
            "\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B0}\u{25B1}\u{25B1}\u{25B1}\u{25B1}\u{25B1}"
        ));
    }
}
