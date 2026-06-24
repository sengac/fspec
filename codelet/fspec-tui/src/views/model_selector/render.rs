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
        let title = self.title_text();
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
        // Title already contains the count; pass it whole with an empty
        // count/suffix via the scaffold's title slot.
        crate::views::full_screen_shell::render_full_screen_scaffold_raw_title(
            area,
            buf,
            &title,
            rows::FOOTER,
            |body_area, buf| {
                self.visible_rows = body_area.height.saturating_sub(1) as usize;
                // Defensive reconcile: now that the real body height is
                // known, re-clamp the offset (covers window-resize and
                // initial-draw where navigation ran with a stale height).
                self.adjust_scroll();
                rows::render_body(
                    body_area,
                    buf,
                    &self.rows,
                    self.loaded,
                    self.selected_index,
                    self.scroll_offset,
                    current.as_deref(),
                );
            },
            None,
        );
    }
}
