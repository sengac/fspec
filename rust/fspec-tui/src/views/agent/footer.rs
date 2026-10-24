//! SessionFooter — 1-row strip painting `<cwd> [⎇ <branch>]` on the
//! right side, and an optional `[compacting: <phase>]` activity chip
//! on the left when a compaction is in flight for the
//! focused session.
//!
//! Features: rpc018-agent-chrome, rpc029-agent-structure-alignment,
//! slash-command-compact (RPC-047).
//!
//! RPC-029: structural rewrite matches the TS Ink original
//! (`src/tui/components/SessionFooter.tsx`). Dark-grey `#333333` row
//! background, 1-column horizontal padding. Right side is two spans:
//! dark-grey cwd (TS `chalk.dim`) + cyan bracketed `⎇` branch suffix.
//! cwd shortening replaces a `$HOME` prefix with `~`.

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

/// SessionFooter widget. Caller owns the `WorkspaceInfo` snapshot AND
/// (RPC-047) the optional `CompactionProgress` for the focused session.
pub struct SessionFooter<'a> {
    pub workspace: Option<&'a WorkspaceInfo>,
    /// When `Some`, the left side paints phase-only compaction activity;
    /// `None` paints only the right-aligned `cwd [⎇ branch]`.
    pub compaction_progress: Option<&'a CompactionProgress>,
    /// RPC-061: per-session pending-supervisor count. When > 0, paints
    /// `[N pending from supervisor]` (yellow) in the left-aligned slot,
    /// suppressing the compaction chip (supervisor signal is more urgent).
    pub supervisor_pending_count: usize,
    /// CONT-002: `⏩ auto-continue (n/N)` indicator while armed; painted in
    /// the left-aligned slot when no supervisor/compaction chip claims it.
    pub continue_indicator: Option<String>,
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
        } else if let Some(indicator) = self.continue_indicator {
            // CONT-002: auto-continue indicator while armed (cyan).
            paint_left_aligned(
                inner,
                buf,
                Line::from(vec![Span::styled(
                    indicator,
                    Style::default().fg(Color::Cyan),
                )]),
            );
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

/// Build the left-aligned activity chip without interpreting legacy counts.
fn build_left_compaction_line(progress: &CompactionProgress) -> Line<'static> {
    Line::from(Span::styled(
        format!("[compacting: {}]", progress.phase),
        Style::default().fg(Color::DarkGray),
    ))
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
///
/// CONT-007: measures DISPLAY width (`Line::width`, unicode-aware) rather
/// than `chrome::line_width`'s char count — the `⏩`/`🎯` continue/goal
/// indicators contain double-width glyphs, and a char-count Rect clipped
/// their closing paren.
fn paint_left_aligned(inner: Rect, buf: &mut Buffer, line: Line<'static>) {
    let width = line.width();
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
#[path = "footer_tests.rs"]
mod tests;
