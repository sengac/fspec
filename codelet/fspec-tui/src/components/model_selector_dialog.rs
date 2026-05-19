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

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;

use codelet_rpc_types::{ProviderInfo, SessionId};

use super::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, MARKER_SELECTED, MARKER_UNSELECTED,
};
use super::model_selector_dialog_rows::{build_rows, ModelSelectorRow};
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
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}

impl ModelSelectorDialog {
    pub fn new(session_id: SessionId, providers: Vec<ProviderInfo>) -> Self {
        let rows = build_rows(&providers);
        let selected_index = rows
            .iter()
            .position(|r| r.selectable)
            .unwrap_or(0);
        Self {
            id: MODEL_SELECTOR_DIALOG_ID.to_string(),
            session_id,
            rows,
            selected_index,
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
        self.selected_index = self
            .rows
            .iter()
            .position(|r| r.selectable)
            .unwrap_or(0);
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
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len();
        let mut next = self.selected_index;
        for _ in 0..len {
            next = if next == 0 { len - 1 } else { next - 1 };
            if self.rows[next].selectable {
                self.selected_index = next;
                return;
            }
        }
    }

    fn move_down(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len();
        let mut next = self.selected_index;
        for _ in 0..len {
            next = (next + 1) % len;
            if self.rows[next].selectable {
                self.selected_index = next;
                return;
            }
        }
    }

    fn build_dialog_rows(&self) -> Vec<DialogRow> {
        if self.rows.is_empty() {
            return vec![DialogRow {
                spans: vec![Span::raw("No providers available".to_string())],
                selectable: false,
                selected: false,
            }];
        }
        self.rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let is_selected = row.selectable && i == self.selected_index;
                let mut spans = Vec::with_capacity(3);
                let marker = if is_selected { MARKER_SELECTED } else { MARKER_UNSELECTED };
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
                DialogRow {
                    spans,
                    selectable: row.selectable,
                    selected: is_selected,
                }
            })
            .collect()
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
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
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
        let session_id = SessionId { value: "test-session".to_string() };
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
