//! RPC-337 — render method for the full-screen ModelSelector mode-view.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Extracted from `mod.rs` to keep that file under the 300-LoC ceiling.
//! Owns the browse-list paint path AND the RPC-344 custom-model overlay
//! (Add / Edit / DeleteConfirm) rendering, both routed through the shared
//! `render_full_screen_scaffold_raw_title` shell.

use super::*;

impl ModelSelectorView {
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let current = self.current_model_id.clone();
        // RPC-344: when a custom-model overlay is active, paint it inside the
        // shell body instead of the browse list.
        let overlay_title = match &self.custom_model_mode {
            CustomModelMode::Add { .. } => Some("Add Custom Model".to_string()),
            CustomModelMode::Edit { .. } => Some("Edit Custom Model".to_string()),
            CustomModelMode::DeleteConfirm { .. } => Some("Delete Custom Model".to_string()),
            CustomModelMode::Browse => None,
        };
        if let Some(shell_title) = overlay_title {
            let mode = self.custom_model_mode.clone();
            let form = self.form.clone();
            // Single footer that fully replaces the browse hints while an
            // overlay is open (TS parity): the form/confirm view owns the
            // footer, so the scaffold's pinned slot carries the overlay hint
            // rather than the stale browse shortcuts.
            let overlay_footer = match &mode {
                CustomModelMode::DeleteConfirm { .. } => form_render::CONFIRM_FOOTER,
                _ => form_render::FORM_FOOTER,
            };
            crate::views::full_screen_shell::render_full_screen_scaffold_raw_title(
                area,
                buf,
                &shell_title,
                overlay_footer,
                |body_area, buf| match &mode {
                    CustomModelMode::Add { profile_name, .. } => {
                        form_render::render_form(
                            body_area,
                            buf,
                            "Add Custom Model",
                            profile_name,
                            &form,
                        );
                    }
                    CustomModelMode::Edit { profile_name, .. } => {
                        form_render::render_form(
                            body_area,
                            buf,
                            "Edit Custom Model",
                            profile_name,
                            &form,
                        );
                    }
                    CustomModelMode::DeleteConfirm {
                        profile_name,
                        display_name,
                        ..
                    } => {
                        form_render::render_delete_confirm(
                            body_area,
                            buf,
                            display_name,
                            profile_name,
                        );
                    }
                    CustomModelMode::Browse => {}
                },
                None,
            );
            return;
        }
        // MODEL-008: render the browse-list title in the two-span style
        // (bold-yellow "Select Model" + dim DarkGray count), matching the
        // provider view. The name + count label both come from the SINGLE
        // source of truth on the view state (`title_name` /
        // `title_count_label`), which `title_text()` also composes — so the
        // rendered UI and the tested string cannot diverge. The refreshing
        // state is a dim status annotation carried inside the count label
        // (never the bold name).
        let title_name = self.title_name();
        let count_label = self.title_count_label();
        crate::views::full_screen_shell::render_full_screen_scaffold_with_title(
            area,
            buf,
            |title_area, buf| {
                crate::views::agent::mode_view_render::render_two_span_title_label(
                    title_area,
                    buf,
                    title_name,
                    &count_label,
                );
            },
            rows::FOOTER,
            |body_area, buf| {
                // MODEL-007: carve the filter-input row off the TOP of the
                // body BEFORE computing visible_rows, mirroring
                // provider_settings/list.rs:200-223. The prompt shows a
                // trailing cursor `_` while typing (filter_mode) and none
                // once the filter is committed; the reserved line shrinks the
                // list area so no model row is hidden behind it.
                let mut body_area = body_area;
                if (self.filter_mode || !self.filter.is_empty()) && body_area.height > 0 {
                    use ratatui::widgets::{Paragraph, Widget};
                    let filter_row = Rect {
                        x: body_area.x,
                        y: body_area.y,
                        width: body_area.width,
                        height: 1,
                    };
                    let prompt = if self.filter_mode {
                        format!("Filter: {}_", self.filter)
                    } else {
                        format!("Filter: {}", self.filter)
                    };
                    Paragraph::new(prompt).render(filter_row, buf);
                    body_area = Rect {
                        x: body_area.x,
                        y: body_area.y + 1,
                        width: body_area.width,
                        height: body_area.height - 1,
                    };
                }
                self.visible_rows = body_area.height.saturating_sub(1) as usize;
                // Defensive reconcile: now that the real body height is
                // known, re-clamp the offset (covers window-resize and
                // initial-draw where navigation ran with a stale height).
                self.adjust_scroll();
                // TUI-101: cache the scrollbar rect for hit-testing.
                let sb_rect = rows::render_body(
                    body_area,
                    buf,
                    &self.rows,
                    self.loaded,
                    self.selected_index,
                    self.scroll_offset,
                    current.as_deref(),
                );
                self.last_scrollbar_rect = sb_rect;
            },
            None,
        );
    }
}
