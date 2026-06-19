//! ModelSelectorDialog — Priority::Foreground modal for picking a
//! provider + model.
//!
//! Feature: spec/features/rpc022-model-selector-dialog.feature
//! Feature: spec/features/rpc027-model-confirm-dialogs.feature
//!
//! RPC-027 renders via the shared dialog_theme renderer. Provider
//! header rows are non-selectable; selected model rows paint with the
//! cyan/black inverse highlight. Capability badges [R]/[V]/[Nk] are
//! rendered DIM on unselected rows so they fade against the model id.

use std::cell::Cell;

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{ProviderInfo, SessionId};

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::model_selector_dialog_rows::{
    build_dialog_rows, build_rows, first_selectable, last_selectable, move_down_skipping_headers,
    move_up_skipping_headers, page_step_selectable, ModelSelectorRow,
};
use super::scroll_viewport::ensure_visible;
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` when the dialog is
/// dismissed from a callback emitted by the dialog's `handle_event`.
pub const MODEL_SELECTOR_DIALOG_ID: &str = "model-selector-dialog";

const FOOTER: &str = "↑↓ Navigate │ Enter Select │ Esc Close\nCustom models: not yet supported";

pub struct ModelSelectorDialog {
    id: String,
    session_id: SessionId,
    rows: Vec<ModelSelectorRow>,
    selected_index: usize,
    scroll_offset: usize,
    last_visible_rows: Cell<usize>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl ModelSelectorDialog {
    pub fn new(session_id: SessionId, providers: Vec<ProviderInfo>) -> Self {
        let rows = build_rows(&providers);
        let selected_index = rows.iter().position(|r| r.selectable).unwrap_or(0);
        Self {
            id: MODEL_SELECTOR_DIALOG_ID.to_string(),
            session_id,
            rows,
            selected_index,
            scroll_offset: 0,
            last_visible_rows: Cell::new(12),
            action_tx: None,
            pending_action: None,
        }
    }

    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
        self.rows = build_rows(&providers);
        self.selected_index = self.rows.iter().position(|r| r.selectable).unwrap_or(0);
        self.scroll_offset = 0;
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    fn visible_rows(&self) -> usize {
        self.last_visible_rows.get().max(1)
    }

    fn ensure_selection_visible(&mut self) {
        let vr = self.visible_rows();
        let total = self.rows.len();
        ensure_visible(&mut self.scroll_offset, self.selected_index, vr, total);
    }

    fn page_step(&mut self, delta: i32) {
        self.selected_index = page_step_selectable(&self.rows, self.selected_index, delta);
        self.ensure_selection_visible();
    }

    fn go_home(&mut self) {
        self.selected_index = first_selectable(&self.rows);
        self.scroll_offset = 0;
    }

    fn go_end(&mut self) {
        self.selected_index = last_selectable(&self.rows);
        self.ensure_selection_visible();
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn provider_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| !r.selectable && r.label.starts_with('▼'))
            .count()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn move_up(&mut self) {
        if let Some(next) = move_up_skipping_headers(&self.rows, self.selected_index) {
            self.selected_index = next;
            self.ensure_selection_visible();
        }
    }

    fn move_down(&mut self) {
        if let Some(next) = move_down_skipping_headers(&self.rows, self.selected_index) {
            self.selected_index = next;
            self.ensure_selection_visible();
        }
    }

    fn build_dialog_rows(&self) -> Vec<DialogRow> {
        build_dialog_rows(
            &self.rows,
            self.selected_index,
            self.scroll_offset,
            self.visible_rows(),
        )
    }
}

impl Component for ModelSelectorDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                KeyCode::Up => {
                    self.move_up();
                    return EventResult::consumed();
                }
                KeyCode::Down => {
                    self.move_down();
                    return EventResult::consumed();
                }
                KeyCode::PageUp => {
                    let vr = self.visible_rows() as i32;
                    self.page_step(-vr);
                    return EventResult::consumed();
                }
                KeyCode::PageDown => {
                    let vr = self.visible_rows() as i32;
                    self.page_step(vr);
                    return EventResult::consumed();
                }
                KeyCode::Home => {
                    self.go_home();
                    return EventResult::consumed();
                }
                KeyCode::End => {
                    self.go_end();
                    return EventResult::consumed();
                }
                KeyCode::Enter => {
                    if let Some(row) = self.rows.get(self.selected_index) {
                        if row.selectable {
                            let action = Action::ModelSelected(
                                self.session_id.clone(),
                                row.provider_key.clone(),
                                row.model_id.clone(),
                            );
                            self.emit_action(action);
                            let id = self.id.clone();
                            let callback: Callback = Box::new(move |compositor| {
                                let _ = compositor.remove(&id);
                            });
                            return EventResult::Consumed(Some(callback));
                        }
                    }
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                _ => {}
            }
        }
        // RPC-028: mouse-wheel navigates the selectable rows.
        if let Event::Mouse(m) = event {
            match m.kind {
                MouseEventKind::ScrollUp => {
                    self.move_up();
                    return EventResult::consumed();
                }
                MouseEventKind::ScrollDown => {
                    self.move_down();
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Body chrome: border + padding + title + gaps + 2-line footer.
        let vr = (area.height as usize).saturating_sub(9).clamp(1, 24);
        self.last_visible_rows.set(vr);
        self.ensure_selection_visible();
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Select Model",
            rows: self.build_dialog_rows(),
            footer: FOOTER,
            min_width: 50,
        };
        render_dialog(area, buf, &dialog);
    }

    fn update(&mut self, action: Action) -> Option<Action> {
        if let Action::ListProvidersLoaded(providers) = action {
            self.set_providers(providers);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn model_selector_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let session_id = SessionId {
            value: "test-session".to_string(),
        };
        let mut dialog = ModelSelectorDialog::new(session_id, Vec::new());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("model_selector_dialog__centered_popup_80x24", rows);
    }
}
