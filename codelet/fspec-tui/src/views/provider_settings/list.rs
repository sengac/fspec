//! RPC-054 — List mode key handling + rendering.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! Extracted from `mod.rs` to keep each file under the 300-LoC ceiling.
//! Owns:
//!   * `handle_list_key` — Esc cascade, /-to-filter mode, clamped arrow
//!     nav (no wrap-around; PgUp/PgDn/Home/End added by RPC-353),
//!     Enter→Detail, `d` opens ConfirmDialog.
//!   * `render_list` — scrollable provider list, filter input line,
//!     `(no providers configured)` placeholder.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::views::agent::confirm_dialog::ConfirmDialog;

use super::{DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// Dispatch a single key in List mode. Filter-mode sub-state is
/// checked FIRST so printable chars accumulate into the filter
/// without triggering the navigation keybinds.
pub(super) fn handle_list_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
) -> ProviderSettingsEvent {
    if view.filter_mode {
        return handle_filter_key(view, key);
    }
    match key.code {
        KeyCode::Esc => {
            // Two-step Esc cascade — clear filter first, close second.
            if !view.filter.is_empty() {
                view.filter.clear();
                view.selected_index = 0;
                view.scroll_offset = 0;
                // RPC-105: filter delta must rebuild nav_items so the
                // header count reactively reflects the cleared filter.
                view.rebuild_nav_items();
                ProviderSettingsEvent::Consumed
            } else {
                ProviderSettingsEvent::Close
            }
        }
        KeyCode::Char('/') => {
            view.filter_mode = true;
            ProviderSettingsEvent::Consumed
        }
        // RPC-353: Page/Home/End paging parity with /model. Reached only
        // when filter_mode is false (checked at the top of handle_list_key),
        // so printable-char accumulation is unaffected. `paged` clamps + clears.
        KeyCode::PageDown => paged(view, view.visible_rows().max(1) as i32),
        KeyCode::PageUp => paged(view, -(view.visible_rows().max(1) as i32)),
        KeyCode::Home => paged(view, i32::MIN / 2),
        KeyCode::End => paged(view, i32::MAX / 2),
        // RPC-160: Tab in List mode emits the new SwitchToModels event.
        // Mirrors TS listModeHandler.ts lines 56-60 `if (key.tab) {
        // onSwitchToModels(); return; }`. No Action is emitted and no
        // view state is mutated — this is a pure UI navigation event
        // that the Navigator translates into a model-settings view
        // transition. filter_mode Tab still falls through
        // handle_filter_key's catch-all (Consumed) because filter_mode
        // is checked at the top of handle_list_key.
        KeyCode::Tab => ProviderSettingsEvent::SwitchToModels,
        KeyCode::Up => {
            // RPC-159: mirror TS contract — clear inline test_result
            // ONLY when navigation actually moves the focus. At index 0
            // (boundary), move_clamped is a no-op and test_result is
            // preserved, matching `if (key.upArrow && selectedIndex > 0)`
            // in src/tui/inputHandlers/listModeHandler.ts.
            let before = view.selected_index;
            view.move_clamped(-1);
            if view.selected_index != before {
                view.clear_test_result();
            }
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Down => {
            // RPC-159: same contract as Up arm — clear test_result only
            // on actual movement, preserving it at the last visible row.
            let before = view.selected_index;
            view.move_clamped(1);
            if view.selected_index != before {
                view.clear_test_result();
            }
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Enter => {
            // PROV-102: when the flat NavItem tree is populated, dispatch
            // SOLELY on the focused NavItem's kind + its own provider_id
            // (see list_actions::enter_on_nav_item). This removes the
            // index-space mismatch where a child row fell through to the
            // legacy visible_providers()[selected_index] path and opened a
            // DIFFERENT provider's Detail.
            if let Some(item) = view.focused_nav_item() {
                let provider_id = item.provider_id.clone();
                let kind = item.kind.clone();
                return super::list_actions::enter_on_nav_item(view, provider_id, kind);
            }
            // Legacy fallback — only reachable when nav_items is empty
            // (pre-RPC-103 callers using `set_providers(...)` alone), where
            // selected_index correctly indexes visible_providers().
            let visible = view.visible_providers();
            let Some(focused) = visible.get(view.selected_index) else {
                return ProviderSettingsEvent::Consumed;
            };
            let pid = focused.provider_id.clone();
            let ctype = focused.credential_type.clone();
            let sub = match ctype.as_str() {
                "oauth" => DetailSub::OAuthNotice,
                _ => DetailSub::Summary { last_status: None },
            };
            view.mode = ProviderSettingsMode::Detail {
                provider_id: pid,
                sub,
            };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // PROV-102: dispatch `d` by the focused NavItem identity when the
            // flat tree is populated, so the delete confirm targets the
            // row's own provider (never a mismatched visible_providers index).
            if view.focused_nav_item().is_some() {
                return super::list_actions::delete_on_nav_item(view);
            }
            // Legacy fallback — nav_items empty (set_providers-only callers).
            let visible = view.visible_providers();
            let Some(focused) = visible.get(view.selected_index) else {
                return ProviderSettingsEvent::Consumed;
            };
            if !focused.configured {
                return ProviderSettingsEvent::Consumed;
            }
            let body = format!("Delete credentials for {}?", focused.provider_id);
            view.delete_confirm = Some(ConfirmDialog::new(
                "Delete credentials?",
                body,
                "Delete",
                None,
                "Cancel",
            ));
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Right => super::list_actions::arrow_expand_collapse(view, true),
        KeyCode::Left => super::list_actions::arrow_expand_collapse(view, false),
        _ => ProviderSettingsEvent::Consumed,
    }
}

/// RPC-353: page/jump the list-mode selection by `delta` (clamped), clearing
/// the inline test_result only on actual movement. Always Consumed.
fn paged(view: &mut ProviderSettingsView, delta: i32) -> ProviderSettingsEvent {
    let before = view.selected_index;
    view.move_clamped(delta);
    if view.selected_index != before {
        view.clear_test_result();
    }
    ProviderSettingsEvent::Consumed
}

fn handle_filter_key(view: &mut ProviderSettingsView, key: KeyEvent) -> ProviderSettingsEvent {
    match key.code {
        KeyCode::Esc => {
            view.filter.clear();
            view.filter_mode = false;
            view.selected_index = 0;
            view.scroll_offset = 0;
            // RPC-105: every filter delta rebuilds the flat nav tree so
            // the header count and rendered rows reflect the new filter
            // synchronously. Mirrors the TS useMemo([providers, filter])
            // reactive dependency.
            view.rebuild_nav_items();
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Enter => {
            view.filter_mode = false;
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Backspace => {
            view.filter.pop();
            view.selected_index = 0;
            view.scroll_offset = 0;
            view.rebuild_nav_items();
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Char(c) => {
            view.filter.push(c);
            view.selected_index = 0;
            view.scroll_offset = 0;
            view.rebuild_nav_items();
            ProviderSettingsEvent::Consumed
        }
        _ => ProviderSettingsEvent::Consumed,
    }
}

pub(super) fn render_list(view: &mut ProviderSettingsView, area: Rect, buf: &mut Buffer) {
    let visible = view.visible_providers();
    // Filter input row (one line, optional)
    let mut body_area = area;
    if (view.filter_mode || !view.filter.is_empty()) && area.height > 0 {
        let filter_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let prompt = if view.filter_mode {
            format!("Filter: {}_", view.filter)
        } else {
            format!("Filter: {}", view.filter)
        };
        Paragraph::new(prompt).render(filter_row, buf);
        body_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height - 1,
        };
    }

    // Prefer the RPC-103 flat NavItem tree when populated (via
    // set_provider_display_infos). Falls back to the legacy
    // visible_providers loop when callers only set the raw
    // ProviderCredentialInfo list (set_providers).
    if !view.nav_items.is_empty() {
        let sb_rect = super::list_nav_render::render_nav_items(view, body_area, buf);
        view.last_scrollbar_rect = sb_rect;
        return;
    }

    if visible.is_empty() {
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

    let visible_rows = body_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let end = (view.scroll_offset + visible_rows).min(visible.len());
    for (row_idx, info) in visible[view.scroll_offset..end].iter().enumerate() {
        let global_idx = view.scroll_offset + row_idx;
        let marker = if global_idx == view.selected_index {
            "▸"
        } else {
            " "
        };
        let configured = if info.configured { "✓" } else { "·" };
        let label = format!(
            " {marker} {} {} — {} models [{}]",
            configured, info.display_name, info.model_count, info.credential_type
        );
        let style = if global_idx == view.selected_index {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default().fg(if info.configured {
                Color::White
            } else {
                Color::Gray
            })
        };
        let y = body_area.y + row_idx as u16;
        let row_area = Rect {
            x: body_area.x,
            y,
            width: body_area.width,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(label, style))).render(row_area, buf);
    }
}
