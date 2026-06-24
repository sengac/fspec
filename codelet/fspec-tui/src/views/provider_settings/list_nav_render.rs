//! RPC-104 — NavItem-driven row paint loop for ProviderSettings list mode.
//!
//! Feature: spec/features/rpc104-provider-settings-row-icons-indents-colors.feature
//!
//! Extracted from `list.rs` to keep that file under the 300-LoC
//! ceiling enforced by `tests/source_shape_rpc054.rs`. Owns:
//!
//! * `render_nav_items` — the scrollable RPC-103 flat-tree paint loop
//!   that dispatches each `NavItem` to `row_render::render_row` with
//!   the kind-appropriate icon / indent / colour band.
//! * `row_kind_and_label` — pure translation from a `NavItem` to a
//!   `(RowKind, display label)` pair. Inline status decorations are
//!   added by follow-up cards (RPC-105 / RPC-107 / RPC-108 / RPC-158).

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Widget};

use super::nav_item::{NavItem, NavItemKind};
use super::row_render::{render_row, row_band_bg, RowKind};
use super::ProviderSettingsView;

pub(super) fn render_nav_items(view: &ProviderSettingsView, body_area: Rect, buf: &mut Buffer) {
    let visible_rows = body_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let nav_items = &view.nav_items;
    if nav_items.is_empty() {
        let mid_y = body_area.y.saturating_add(body_area.height / 2);
        let row = Rect {
            x: body_area.x,
            y: mid_y,
            width: body_area.width,
            height: 1,
        };
        Paragraph::new("(no providers configured)")
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }
    let end = (view.scroll_offset + visible_rows).min(nav_items.len());
    for (row_idx, item) in nav_items[view.scroll_offset..end].iter().enumerate() {
        let global_idx = view.scroll_offset + row_idx;
        let selected = global_idx == view.selected_index;
        let (kind, label) = row_kind_and_label(item, view);
        let y = body_area.y + row_idx as u16;
        let row_area = Rect {
            x: body_area.x,
            y,
            width: body_area.width,
            height: 1,
        };
        let end_x = render_row(kind, &label, selected, row_area, buf);
        // RPC-158: paint the inline test-result decoration on Provider
        // header rows whose provider_id matches `view.test_result`. The
        // decoration is appended after the label with a single ASCII
        // space separator; foreground comes from the status, background
        // matches the row's existing colour band.
        if matches!(kind, RowKind::Provider { .. }) {
            if let Some(test_result) = view.test_result.as_ref() {
                if test_result.provider_id == item.provider_id {
                    paint_test_result_decoration(
                        kind,
                        selected,
                        &test_result.status,
                        row_area,
                        end_x,
                        buf,
                    );
                }
            }
        }
    }
}

fn paint_test_result_decoration(
    kind: RowKind,
    selected: bool,
    status: &super::ProviderTestStatus,
    row_area: Rect,
    end_x: u16,
    buf: &mut Buffer,
) {
    // Right boundary (exclusive) of the row's painted area.
    let right_bound = row_area.x.saturating_add(row_area.width);
    if end_x >= right_bound {
        return;
    }
    // Reserve one cell for the separator space.
    let separator_x = end_x;
    let decoration_x = end_x.saturating_add(1);
    if decoration_x >= right_bound {
        return;
    }
    let (text, fg) = status.decoration();
    let bg = row_band_bg(kind, selected);
    let style = Style::default().fg(fg).bg(bg);
    let remaining = (right_bound - decoration_x) as usize;
    // Paint the separator space with the band background only — no fg
    // change needed since it's whitespace.
    buf[(separator_x, row_area.y)].set_symbol(" ");
    buf[(separator_x, row_area.y)].set_style(Style::default().bg(bg));
    // Paint the decoration text on top of the existing band.
    buf.set_stringn(decoration_x, row_area.y, &text, remaining, style);
}

fn row_kind_and_label(item: &NavItem, view: &ProviderSettingsView) -> (RowKind, String) {
    let display = view
        .display_providers
        .iter()
        .find(|p| p.id == item.provider_id);
    match &item.kind {
        NavItemKind::Provider { expanded } => {
            let name = display
                .map(|p| p.name.clone())
                .unwrap_or_else(|| item.provider_id.clone());
            // PROV-098: append the credential annotation to the provider
            // header (TS ProviderSettingsPanel.tsx:594-608). Driven SOLELY
            // by the display masked_key/source — OAuth status stays its own
            // child row.
            let label = format!("{name}{}", provider_annotation(display));
            (
                RowKind::Provider {
                    expanded: *expanded,
                },
                label,
            )
        }
        NavItemKind::Profile { profile_name } => (RowKind::Profile, profile_name.clone()),
        NavItemKind::AddProfile => (RowKind::AddProfile, "Add Profile".to_string()),
        NavItemKind::ApiKey => {
            // PROV-098: append the credential annotation to the ApiKey
            // child row (TS ProviderSettingsPanel.tsx:734-746). Empty
            // state is "(not set)" here, distinct from the provider row's
            // "(not configured)".
            let label = format!("API Key{}", api_key_annotation(display));
            (RowKind::ApiKey, label)
        }
        NavItemKind::OAuthLogin { label, .. } => (RowKind::OauthLogin, label.clone()),
        NavItemKind::OAuthStatus { label } => (RowKind::OauthStatus, label.clone()),
    }
}

/// PROV-098: credential suffix for a provider header row.
/// `" ✓ {masked} [{source}]"` when a key is present (` [{source}]` only
/// when source is Some), else `" (not configured)"`.
fn provider_annotation(display: Option<&super::nav_item::ProviderDisplayInfo>) -> String {
    match display.and_then(|p| p.masked_key.as_deref()) {
        Some(masked) => format!(" ✓ {masked}{}", source_suffix(display)),
        None => " (not configured)".to_string(),
    }
}

/// PROV-098: credential suffix for an ApiKey child row. Same shape as the
/// provider row but the empty state is `" (not set)"`.
fn api_key_annotation(display: Option<&super::nav_item::ProviderDisplayInfo>) -> String {
    match display.and_then(|p| p.masked_key.as_deref()) {
        Some(masked) => format!(" ✓ {masked}{}", source_suffix(display)),
        None => " (not set)".to_string(),
    }
}

/// `" [{source}]"` when the display info carries a source, else empty.
fn source_suffix(display: Option<&super::nav_item::ProviderDisplayInfo>) -> String {
    match display.and_then(|p| p.source.as_deref()) {
        Some(source) => format!(" [{source}]"),
        None => String::new(),
    }
}
