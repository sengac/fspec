//! RPC-344 — overlay rendering for the custom-model add/edit form and the
//! delete confirmation.
//!
//! Feature: spec/features/model-selector-custom-model-crud.feature
//!
//! Presentational only (mirrors the TS `CustomModelFormView.tsx` /
//! `DeleteCustomModelConfirmView.tsx`): paints into the body area handed in
//! by the parent `ModelSelectorView::render`. All state arrives by value.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::form::{CustomModelForm, FieldType, FORM_FIELDS};

/// Footer hint for the add/edit form overlay. Rendered ONCE by the parent
/// scaffold's pinned footer slot (not inside the body) so the browse footer is
/// fully replaced while a form is open — parity with the TS `CustomModelFormView`
/// which early-returns in place of the browse view.
pub(super) const FORM_FOOTER: &str =
    "↑↓ navigate fields | ←→ cycle options | Enter save | Esc cancel";
/// Footer hint for the delete-confirmation overlay (see [`FORM_FOOTER`]).
pub(super) const CONFIRM_FOOTER: &str = "y/Enter confirm delete | n/Esc cancel";

/// Display value for a field given the current form values.
fn field_value(form: &CustomModelForm, idx: usize) -> String {
    match idx {
        0 => form.id.clone(),
        1 => form.display_name.clone(),
        2 => form.facade.clone().unwrap_or_default(),
        3 => form.context_window.clone(),
        4 => form.max_output_tokens.clone(),
        5 => form.compaction_trigger.clone(),
        6 => form.reasoning.map(|b| b.to_string()).unwrap_or_default(),
        7 => form.has_vision.map(|b| b.to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Paint the add/edit form overlay: title, profile line, the eight fields
/// with the active one highlighted, and a footer hint.
pub(super) fn render_form(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    profile_name: &str,
    form: &CustomModelForm,
) {
    if area.height == 0 {
        return;
    }
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" — profile: {profile_name}"),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]));
    lines.push(Line::from(""));

    for (idx, field) in FORM_FIELDS.iter().enumerate() {
        let active = idx == form.field_index;
        let marker = if active { "> " } else { "  " };
        let base = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default()
        };
        let mut spans: Vec<Span<'static>> = vec![Span::styled(
            format!("{marker}{}", field.label),
            base.add_modifier(if active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        )];
        if field.required {
            spans.push(Span::styled(
                "*".to_string(),
                if active {
                    base
                } else {
                    Style::default().fg(Color::Red)
                },
            ));
        }
        spans.push(Span::styled(": ".to_string(), base));
        let value = field_value(form, idx);
        if value.is_empty() {
            spans.push(Span::styled(
                field.placeholder.to_string(),
                if active {
                    base
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                },
            ));
        } else {
            spans.push(Span::styled(value, base));
        }
        if active && field.field_type == FieldType::Select {
            spans.push(Span::styled(
                format!(" (←/→ to cycle: {})", field.options.join(", ")),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
        if active && field.field_type == FieldType::Boolean {
            spans.push(Span::styled(
                " (←/→ to toggle)".to_string(),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
        lines.push(Line::from(spans));
    }

    Paragraph::new(lines).render(area, buf);
}

/// Paint the delete confirmation overlay.
pub(super) fn render_delete_confirm(
    area: Rect,
    buf: &mut Buffer,
    display_name: &str,
    profile_name: &str,
) {
    if area.height == 0 {
        return;
    }
    let lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            "Delete Custom Model".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("Are you sure you want to delete "),
            Span::styled(
                display_name.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" from profile "),
            Span::styled(
                profile_name.to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("?"),
        ]),
    ];
    Paragraph::new(lines).render(area, buf);
}
