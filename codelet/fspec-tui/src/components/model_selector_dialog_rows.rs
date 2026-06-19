//! Helpers extracted from `model_selector_dialog.rs` so the parent
//! file stays under the 300-LoC budget.
//!
//! Feature: spec/features/rpc022-model-selector-dialog.feature
//! Feature: spec/features/rpc027-model-confirm-dialogs.feature
//!
//! RPC-027 update: `DialogBody` adapter removed; rows now flow into
//! the shared `dialog_theme` renderer instead of `tui_popup`.
//! RPC-028 update: viewport windowing math + `build_dialog_rows`
//! lifted out of the parent file.

use codelet_rpc_types::ProviderInfo;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use super::dialog_theme::{DialogRow, MARKER_SELECTED, MARKER_UNSELECTED};

/// One displayable row in the dialog. Provider headers are not
/// selectable (`selectable = false`); model rows are.
///
/// RPC-337: re-scoped from `pub(super)` to `pub(crate)` so the new
/// full-screen `views/model_selector/` mode-view can reuse the row
/// projection + header-skipping navigation helpers.
#[derive(Debug, Clone)]
pub(crate) struct ModelSelectorRow {
    /// Render text for the row (without the leading `▸ ` marker).
    pub(crate) label: String,
    /// Capability/context badge suffix (e.g. `" [R] [V] [200k]"`),
    /// rendered DIM on unselected rows.
    pub(crate) badges: String,
    /// True for model rows (Enter emits ModelSelected); false for
    /// provider headers.
    pub(crate) selectable: bool,
    /// Provider key — populated only when `selectable` is true.
    pub(crate) provider_key: String,
    /// Model id — populated only when `selectable` is true.
    pub(crate) model_id: String,
    /// RPC-338: true for a local-server profile section header (drives the
    /// magenta 📁 icon). Header rows only; always false for model rows.
    pub(crate) is_profile: bool,
    /// RPC-338: true when an (unreachable) profile header (drives the red
    /// `(unreachable)` marker). Header rows only.
    pub(crate) is_unreachable: bool,
}

/// Flatten a `[ProviderInfo]` list into a flat `[ModelSelectorRow]`
/// projection: a `▼ provider_name` header followed by each model row.
pub(crate) fn build_rows(providers: &[ProviderInfo]) -> Vec<ModelSelectorRow> {
    let mut rows = Vec::with_capacity(providers.len() * 4);
    for provider in providers {
        rows.push(ModelSelectorRow {
            label: format!(
                "▼ {} ({} models)",
                provider.display_name,
                provider.models.len()
            ),
            badges: String::new(),
            selectable: false,
            provider_key: String::new(),
            model_id: String::new(),
            is_profile: provider.profile_name.is_some(),
            is_unreachable: provider.is_unreachable,
        });
        for model in &provider.models {
            let label = model.display_name.clone();
            let mut badges = String::new();
            if model.supports_reasoning {
                badges.push_str(" [R]");
            }
            if model.supports_vision {
                badges.push_str(" [V]");
            }
            if model.context_window > 0 {
                let cw = if model.context_window >= 1_000 {
                    format!("{}k", model.context_window / 1_000)
                } else {
                    model.context_window.to_string()
                };
                badges.push_str(&format!(" [{cw}]"));
            }
            rows.push(ModelSelectorRow {
                label,
                badges,
                selectable: true,
                provider_key: provider.key.clone(),
                model_id: model.id.clone(),
                is_profile: false,
                is_unreachable: false,
            });
        }
    }
    rows
}

/// Window the flat `rows` list into the visible viewport and convert
/// each visible entry into a `DialogRow`. Inserts `↑`/`↓` indicator
/// rows on the top/bottom visible rows when the list overflows the
/// window (mirrors `views/board/viewport.rs:95-101`).
///
/// `selected_index` is the global (unwindowed) selection index;
/// rows that match it AND are selectable receive the inverse highlight
/// via `DialogRow.selected = true`.
pub(super) fn build_dialog_rows(
    rows: &[ModelSelectorRow],
    selected_index: usize,
    scroll_offset: usize,
    visible_rows: usize,
) -> Vec<DialogRow> {
    if rows.is_empty() {
        return vec![DialogRow {
            spans: vec![Span::raw("No providers available".to_string())],
            selectable: false,
            selected: false,
        }];
    }
    let total = rows.len();
    let so = scroll_offset.min(total.saturating_sub(1));
    let vr = visible_rows.max(1);
    let up_arrow = so > 0;
    let down_arrow = so + vr < total;
    let end = (so + vr).min(total);
    let mut out: Vec<DialogRow> = Vec::with_capacity(vr);
    for (rel, abs_i) in (so..end).enumerate() {
        let is_first_visible = rel == 0;
        let is_last_visible = rel + 1 == end - so;
        if up_arrow && is_first_visible {
            out.push(DialogRow {
                spans: vec![Span::styled(
                    "↑".to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                )],
                selectable: false,
                selected: false,
            });
            continue;
        }
        if down_arrow && is_last_visible {
            out.push(DialogRow {
                spans: vec![Span::styled(
                    "↓".to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                )],
                selectable: false,
                selected: false,
            });
            continue;
        }
        let row = &rows[abs_i];
        let is_selected = row.selectable && abs_i == selected_index;
        let mut spans = Vec::with_capacity(3);
        let marker = if is_selected {
            MARKER_SELECTED
        } else {
            MARKER_UNSELECTED
        };
        if row.selectable {
            spans.push(Span::raw(marker.to_string()));
            spans.push(Span::raw(row.label.clone()));
            if !row.badges.is_empty() {
                let badge_style = if is_selected {
                    Style::default()
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };
                spans.push(Span::styled(row.badges.clone(), badge_style));
            }
        } else {
            spans.push(Span::raw(row.label.clone()));
        }
        out.push(DialogRow {
            spans,
            selectable: row.selectable,
            selected: is_selected,
        });
    }
    out
}

/// Find the next selectable index when moving up. Skips non-selectable
/// header rows; wraps around the ends. Returns `None` when no row is
/// selectable.
pub(crate) fn move_up_skipping_headers(rows: &[ModelSelectorRow], current: usize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let len = rows.len();
    let mut next = current;
    for _ in 0..len {
        next = if next == 0 { len - 1 } else { next - 1 };
        if rows[next].selectable {
            return Some(next);
        }
    }
    None
}

/// Find the next selectable index when moving down. Mirror of
/// `move_up_skipping_headers`.
pub(crate) fn move_down_skipping_headers(
    rows: &[ModelSelectorRow],
    current: usize,
) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let len = rows.len();
    let mut next = current;
    for _ in 0..len {
        next = (next + 1) % len;
        if rows[next].selectable {
            return Some(next);
        }
    }
    None
}

/// PageUp/PageDown step. Clamps `selected_index + delta` to
/// `[0, len-1]`, then walks in `delta`'s sign direction until it lands
/// on a selectable row, falling back to the first/last selectable row
/// at the edges.
pub(crate) fn page_step_selectable(
    rows: &[ModelSelectorRow],
    selected_index: usize,
    delta: i32,
) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let total = rows.len() as i32;
    let mut next = (selected_index as i32 + delta).clamp(0, total - 1);
    let step: i32 = if delta < 0 { -1 } else { 1 };
    while !rows[next as usize].selectable {
        next += step;
        if next < 0 || next >= total {
            next = if step < 0 {
                rows.iter().position(|r| r.selectable).unwrap_or(0) as i32
            } else {
                rows.iter()
                    .rposition(|r| r.selectable)
                    .unwrap_or((total - 1) as usize) as i32
            };
            break;
        }
    }
    next as usize
}

/// First selectable row index (Home).
pub(crate) fn first_selectable(rows: &[ModelSelectorRow]) -> usize {
    rows.iter().position(|r| r.selectable).unwrap_or(0)
}

/// Last selectable row index (End).
pub(crate) fn last_selectable(rows: &[ModelSelectorRow]) -> usize {
    rows.iter()
        .rposition(|r| r.selectable)
        .unwrap_or(rows.len().saturating_sub(1))
}
