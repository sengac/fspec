//! PROV-110 — overlay rendering for the profile create/edit form.
//!
//! Feature: spec/features/provider-settings-profile-form.feature
//!
//! Presentational only (mirrors the TS profile form view): paints into the
//! body area handed in by `ProviderSettingsView::render`. All state arrives by
//! reference. The active field (or the name, while editing it) is highlighted.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::profile_form::{ProfileForm, PROFILE_FORM_FIELDS};

/// Footer hint shown while a profile form is open (TUI-084 arrow navigation).
pub(super) const FORM_FOOTER: &str = "↑/↓: switch field · Enter: save · Esc: cancel";

fn active_style() -> Style {
    Style::default().fg(Color::Black).bg(Color::Cyan)
}

/// Build the name line, highlighting it while it is being edited.
fn name_line(form: &ProfileForm) -> Line<'static> {
    let active = form.is_editing_name;
    let base = if active {
        active_style().add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let marker = if active { "> " } else { "  " };
    let value = if form.name.is_empty() {
        Span::styled(
            "(profile name)".to_string(),
            Style::default().add_modifier(Modifier::DIM),
        )
    } else {
        Span::styled(
            form.name.clone(),
            if active {
                active_style()
            } else {
                Style::default()
            },
        )
    };
    Line::from(vec![Span::styled(format!("{marker}Name: "), base), value])
}

/// Build one connection-field line.
fn field_line(form: &ProfileForm, idx: usize, label: &str) -> Line<'static> {
    let active = !form.is_editing_name && idx == form.field_index;
    let base = if active {
        active_style().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let marker = if active { "> " } else { "  " };
    let value = form.field_value(idx);
    let value_span = if value.is_empty() {
        Span::styled(
            "(empty)".to_string(),
            Style::default().add_modifier(Modifier::DIM),
        )
    } else {
        Span::styled(
            value.to_string(),
            if active {
                active_style()
            } else {
                Style::default()
            },
        )
    };
    Line::from(vec![
        Span::styled(format!("{marker}{label}: "), base),
        value_span,
    ])
}

/// Paint the profile create/edit form: title, the name line, and the five
/// connection fields with the active one highlighted.
pub(super) fn render_form(area: Rect, buf: &mut Buffer, title: &str, form: &ProfileForm) {
    if area.height == 0 {
        return;
    }
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(name_line(form));
    for (idx, label) in PROFILE_FORM_FIELDS.iter().enumerate() {
        lines.push(field_line(form, idx, label));
    }
    Paragraph::new(lines).render(area, buf);
}
