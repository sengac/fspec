//! RPC-028 — render helpers for ResumeSessionView and
//! SearchHistoryView. Extracted to keep the parent files under the
//! 300-LoC source-shape budget.
//!
//! RPC-054 (revision): `render_title_with_count` and
//! `render_footer_hint` are promoted to `pub(crate)` so the
//! ProviderSettingsView can reuse them. The title helper takes a
//! `suffix` parameter so different views can label the count
//! correctly (sessions: "available", providers: "configured").
//!
//! TUI-096: `render_session_rows` now renders a 2-line format per
//! session (name line + detail line) with `format_time_ago` helper.

use codelet_rpc_types::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub(crate) fn render_title_with_count(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    count: usize,
    suffix: &str,
) {
    let text = format!("{title} ({count} {suffix})");
    let style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
}

/// RPC-350 R1 — provider-specific two-span title: the name segment in
/// **bold yellow** and the ` ({count} {suffix})` segment in dim gray
/// (`Color::DarkGray`). Mirrors `ProviderSettingsPanel.tsx:550-555`
/// (`<Text bold color="yellow">…</Text><Text dimColor> (N items)</Text>`).
///
/// This is intentionally SEPARATE from [`render_title_with_count`] so the
/// shared blue-bold title used by ResumeSession / SearchHistory / model
/// views is never affected (RPC-350 R5 guard). Wired into the provider
/// view via `render_full_screen_scaffold_with_title`.
pub(crate) fn render_two_span_title(
    area: Rect,
    buf: &mut Buffer,
    name: &str,
    count: usize,
    suffix: &str,
) {
    let name_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let count_style = Style::default().fg(Color::DarkGray);
    let line = Line::from(vec![
        Span::styled(name.to_string(), name_style),
        Span::styled(format!(" ({count} {suffix})"), count_style),
    ]);
    Paragraph::new(line).render(area, buf);
}

/// MODEL-008 — label-based two-span title: the `name` segment in **bold
/// yellow** and a ` {label}` segment in dim gray (`Color::DarkGray`),
/// where `label` is a PRE-BUILT, already-pluralized string (e.g.
/// `"(1 model)"` / `"(12 models)"` / `"(12 models) (refreshing...)"`).
///
/// Sibling of [`render_two_span_title`] (which takes `count`+`suffix` and
/// cannot pluralize): the model view builds the label from the single
/// source of truth `rows::model_count_label`, so the singular/plural rule
/// lives in exactly one place and the provider view's `"items"` path is
/// left untouched.
pub(crate) fn render_two_span_title_label(area: Rect, buf: &mut Buffer, name: &str, label: &str) {
    let name_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let count_style = Style::default().fg(Color::DarkGray);
    let line = Line::from(vec![
        Span::styled(name.to_string(), name_style),
        Span::styled(format!(" {label}"), count_style),
    ]);
    Paragraph::new(line).render(area, buf);
}

pub(crate) fn render_footer_hint(area: Rect, buf: &mut Buffer, text: &str) {
    Paragraph::new(text.to_string()).render(area, buf);
}

/// TUI-096: Format a time delta (in seconds) as a human-readable "time ago"
/// string. Thresholds:
///   < 60s       → "just now"
///   < 60m       → "{m}m ago"
///   < 24h       → "{h}h ago"
///   < 7d        → "{d}d ago"
///   < 30d       → "{w}w ago"
///   >= 30d      → "{mo}mo ago"
pub fn format_time_ago(seconds: i64) -> String {
    if seconds < 60 {
        return "just now".to_string();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{days}d ago");
    }
    let weeks = days / 7;
    if weeks < 4 {
        return format!("{weeks}w ago");
    }
    let months = weeks / 4;
    format!("{months}mo ago")
}

/// TUI-096: Render session rows in a 2-line format per session.
///
/// Each session occupies 2 visual rows:
///   Line 1: "▸ Session Name" (or "  Session Name" if unselected)
///   Line 2: "    N messages | provider/model | time ago"
///
/// Selected rows use REVERSED+BOLD style for both lines.
///
/// TUI-097: When session count exceeds visible area, a proportional
/// scrollbar is rendered on the rightmost column and content width
/// is reduced by 1 column.
pub fn render_session_rows(
    area: Rect,
    buf: &mut Buffer,
    sessions: &[SessionInfo],
    selected_index: usize,
    scroll_offset: usize,
) {
    if sessions.is_empty() {
        let mid_y = area.y.saturating_add(area.height / 2);
        let row = Rect {
            x: area.x,
            y: mid_y,
            width: area.width,
            height: 1,
        };
        Paragraph::new("(no sessions to resume)")
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }

    let visible_rows = area.height as usize;
    if visible_rows == 0 {
        return;
    }

    // TUI-096: Each session takes 2 visual rows
    let visible_sessions = visible_rows / 2;

    // TUI-097: Determine if scrollbar is needed
    let show_scrollbar = sessions.len() > visible_sessions;
    let content_width = if show_scrollbar {
        area.width.saturating_sub(1)
    } else {
        area.width
    };

    let end = (scroll_offset + visible_sessions).min(sessions.len());

    for (session_idx, info) in sessions[scroll_offset..end].iter().enumerate() {
        let global_idx = scroll_offset + session_idx;
        let is_selected = global_idx == selected_index;

        let style = if is_selected {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };

        // Line 1: Name line with selection marker
        let marker = if is_selected { "▸" } else { " " };
        let name_line = format!(" {marker} {}", info.name);
        let y1 = area.y + (session_idx * 2) as u16;
        let row_area1 = Rect {
            x: area.x,
            y: y1,
            width: content_width,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(name_line, style))).render(row_area1, buf);

        // Line 2: Detail line with message count, provider, time ago
        let provider_str = info
            .provider_id
            .as_deref()
            .map(|p| {
                if let Some(model) = &info.model_id {
                    format!("{p}/{model}")
                } else {
                    p.to_string()
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        let time_str = info
            .updated_at_ms
            .map(|ts| {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                let delta = now_ms - ts;
                if delta < 0 {
                    "just now".to_string()
                } else {
                    format_time_ago(delta / 1000)
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        let detail_line = format!(
            "    {} messages | {} | {}",
            info.message_count, provider_str, time_str
        );
        let y2 = y1 + 1;
        let row_area2 = Rect {
            x: area.x,
            y: y2,
            width: content_width,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(detail_line, style))).render(row_area2, buf);
    }

    // TUI-097: Render proportional scrollbar when overflow
    if show_scrollbar {
        crate::components::list_scrollbar::render_list_scrollbar(
            Rect {
                x: area.x + content_width,
                y: area.y,
                width: 1,
                height: area.height,
            },
            buf,
            scroll_offset,
            visible_sessions,
            sessions.len(),
        );
    }
}
