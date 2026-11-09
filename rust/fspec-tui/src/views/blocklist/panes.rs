//! RPC-056 / BLOCK-008 / BLOCK-009 — BlocklistView pane painters.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-view-scrolling.feature
//! Feature: spec/features/blocklist-view-framing.feature
//!
//! Free functions painting the windowed left list pane (scrollbar gutter
//! plus `Showing X-Y of N`), the right details pane, and the empty-state
//! placeholder. Split out of `render.rs` so every file stays under 300
//! lines; called by `BlocklistView::render` in `render.rs`.
//!
//! TUI-131 (spec/features/long-text-wraps-instead-of-being-cut-off.feature):
//! the details pane wraps every field to the pane width.

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use unicode_width::UnicodeWidthStr;

use super::{action_color, derive_category};
use crate::views::agent::text_wrap::{wrap_rows_hanging, wrapped_lines};
use crate::views::diff_common::render_pane_scrollbar;

/// Render the windowed left list pane. Reconciles `scroll_offset` /
/// `visible_rows` from the real pane height, paints only the visible row
/// slice, an overflow scrollbar gutter, and the `Showing X-Y of N`
/// indicator (BLOCK-008).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_left_pane(
    area: Rect,
    buf: &mut Buffer,
    rules: &[codelet_rpc_types::BlocklistRuleInfo],
    selected_index: usize,
    scroll_offset: &mut usize,
    visible_rows: &mut usize,
    session_disabled: &HashSet<String>,
) {
    // Reserve the bottom line for the `Showing X-Y of N` indicator so
    // no rule row is hidden behind it.
    let has_indicator = area.height > 1;
    let list_area = if has_indicator {
        Rect {
            height: area.height.saturating_sub(1),
            ..area
        }
    } else {
        area
    };
    let indicator_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    // Each rule occupies 2 rows (id line + meta line).
    let rows = (list_area.height as usize) / 2;
    *visible_rows = rows;
    // Defensive reconcile now the real body height is known.
    crate::components::scroll_viewport::ensure_visible(
        scroll_offset,
        selected_index,
        rows,
        rules.len(),
    );
    let total = rules.len();
    let overflow = total > rows && rows > 0;
    let list_width = if overflow {
        list_area.width.saturating_sub(1)
    } else {
        list_area.width
    };
    let end = if rows == 0 {
        total
    } else {
        (*scroll_offset + rows).min(total)
    };
    let start = (*scroll_offset).min(end);

    let mut lines: Vec<Line<'_>> = Vec::with_capacity((end - start) * 2);
    for (idx, rule) in rules.iter().enumerate().take(end).skip(start) {
        let selected = idx == selected_index;
        let disabled = session_disabled.contains(&rule.id);
        let glyph = if disabled { '○' } else { '●' };
        let prefix = if selected { "> " } else { "  " };

        let row_style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if disabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let id_line = Line::from(vec![
            Span::raw(prefix),
            Span::styled(format!("{glyph} {}", rule.id), row_style),
        ]);
        lines.push(id_line);

        let action_color = action_color(&rule.action);
        let category = derive_category(&rule.pattern);
        let mut meta_spans: Vec<Span<'_>> = vec![
            Span::raw("    "),
            Span::styled(
                format!("[{}]", rule.action),
                Style::default().fg(action_color),
            ),
            Span::raw(" "),
            Span::styled(format!("[{category}]"), Style::default().fg(Color::Magenta)),
            Span::raw(" "),
            Span::styled(rule.source.clone(), Style::default().fg(Color::DarkGray)),
        ];
        if disabled {
            meta_spans.push(Span::styled(
                " (disabled)",
                Style::default().fg(Color::Yellow),
            ));
        }
        lines.push(Line::from(meta_spans));
    }
    let paint_area = Rect {
        width: list_width,
        ..list_area
    };
    Paragraph::new(lines).render(paint_area, buf);

    if overflow {
        render_pane_scrollbar(list_area, buf, list_width, *scroll_offset, rows, total);
    }

    if has_indicator {
        let first = if total == 0 { 0 } else { start + 1 };
        let indicator = format!("Showing {first}-{end} of {total}");
        Paragraph::new(Line::from(Span::styled(
            indicator,
            Style::default().fg(Color::DarkGray),
        )))
        .render(indicator_area, buf);
    }
}

/// TUI-131: `label` (white) then `value`, wrapped to `width` with the
/// value's first row after the label, so a long value is shown whole.
fn field_lines(label: &str, value: &str, value_style: Style, width: usize) -> Vec<Line<'static>> {
    let first_width = width.saturating_sub(label.width());
    wrap_rows_hanging(value, first_width, width)
        .into_iter()
        .enumerate()
        .map(|(row, (text, _))| {
            let value = Span::styled(text, value_style);
            if row == 0 {
                let label = Span::styled(label.to_string(), Style::default().fg(Color::White));
                Line::from(vec![label, value])
            } else {
                Line::from(value)
            }
        })
        .collect()
}

/// TUI-131: a white `heading` row, then `text` wrapped to `width`.
fn section_lines(heading: &str, text: &str, style: Style, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            heading.to_string(),
            Style::default().fg(Color::White),
        )),
    ];
    lines.extend(wrapped_lines(text, width, style));
    lines
}

/// Paint the details of the `focused` rule. TUI-131: every field is
/// wrapped to the pane width (`text_wrap`), so a long pattern, reason or
/// guidance is shown whole instead of being cut at the pane edge.
pub(super) fn render_right_pane(
    area: Rect,
    buf: &mut Buffer,
    focused: Option<&codelet_rpc_types::BlocklistRuleInfo>,
    session_disabled: &HashSet<String>,
) {
    let width = usize::from(area.width);
    let mut lines: Vec<Line<'static>> = Vec::new();
    if let Some(rule) = focused {
        lines.push(Line::from(Span::styled(
            "Rule Details",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        let category = derive_category(&rule.pattern).to_string();
        let action = Style::default().fg(action_color(&rule.action));
        let dim = Style::default().fg(Color::DarkGray);
        let magenta = Style::default().fg(Color::Magenta);
        lines.extend(field_lines("ID: ", &rule.id, Style::default(), width));
        lines.extend(field_lines("Action: ", &rule.action, action, width));
        lines.extend(field_lines("Source: ", &rule.source, dim, width));
        lines.extend(field_lines("Category: ", &category, magenta, width));
        lines.extend(section_lines("Pattern:", &rule.pattern, dim, width));
        if !rule.reason.is_empty() {
            lines.extend(section_lines(
                "Reason:",
                &rule.reason,
                Style::default(),
                width,
            ));
        }
        if let Some(g) = rule.guidance.as_ref().filter(|g| !g.is_empty()) {
            let green = Style::default().fg(Color::Green);
            lines.extend(section_lines("Guidance:", g, green, width));
        }
        lines.push(Line::from(""));
        let disabled = session_disabled.contains(&rule.id);
        let (status_color, status_text) = if disabled {
            (Color::Yellow, "disabled (session)")
        } else {
            (Color::Green, "enabled")
        };
        let status = Style::default().fg(status_color);
        lines.extend(field_lines("Session Status: ", status_text, status, width));
    } else {
        lines.push(Line::from(Span::styled(
            "(no rule selected)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    Paragraph::new(lines).render(area, buf);
}

pub(super) fn render_empty(area: Rect, buf: &mut Buffer) {
    let lines: Vec<Line<'_>> = vec![
        Line::from(Span::styled(
            "No blocklist rules configured.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Blocklist rules prevent dangerous commands and guide",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "AI agents to use proper tools and patterns.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "System config: ~/.fspec/blocklist.json",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Project config: .fspec/blocklist.json",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}
