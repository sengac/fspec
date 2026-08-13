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

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::{action_color, derive_category};
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

pub(super) fn render_right_pane(
    area: Rect,
    buf: &mut Buffer,
    focused: Option<&codelet_rpc_types::BlocklistRuleInfo>,
    session_disabled: &HashSet<String>,
) {
    let mut lines: Vec<Line<'_>> = Vec::new();
    if let Some(rule) = focused {
        lines.push(Line::from(Span::styled(
            "Rule Details",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::White)),
            Span::raw(rule.id.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Action: ", Style::default().fg(Color::White)),
            Span::styled(
                rule.action.clone(),
                Style::default().fg(action_color(&rule.action)),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::White)),
            Span::styled(rule.source.clone(), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Category: ", Style::default().fg(Color::White)),
            Span::styled(
                derive_category(&rule.pattern).to_string(),
                Style::default().fg(Color::Magenta),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Pattern:",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(Span::styled(
            rule.pattern.clone(),
            Style::default().fg(Color::DarkGray),
        )));
        if !rule.reason.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Reason:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(rule.reason.clone()));
        }
        if let Some(g) = rule.guidance.as_ref() {
            if !g.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Guidance:",
                    Style::default().fg(Color::White),
                )));
                lines.push(Line::from(Span::styled(
                    g.clone(),
                    Style::default().fg(Color::Green),
                )));
            }
        }
        lines.push(Line::from(""));
        let disabled = session_disabled.contains(&rule.id);
        let (status_color, status_text) = if disabled {
            (Color::Yellow, "disabled (session)")
        } else {
            (Color::Green, "enabled")
        };
        lines.push(Line::from(vec![
            Span::styled("Session Status: ", Style::default().fg(Color::White)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]));
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
