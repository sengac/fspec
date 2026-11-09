//! MuxConfigDialog — Priority::Foreground modal for configuring the mux
//! layout (MUX-004, MUX-010).
//!
//! Feature: spec/features/mux-config-dialog.feature +
//! spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! Mounted by the bare `/mux` slash command, the `/mux` slash-popup pick
//! and the board 'M' key via `App::handle_open_mux_config_dialog`. The
//! dialog edits a DRAFT of the live [`MuxConfig`]; the live grid is
//! untouched until commit (R5 cancel-safe).
//!
//! MUX-010 body: the layout-string field, the exact parse / fit error
//! under it, a live drawn preview of the panes (boxes labelled with their
//! pane kind, sized by weight; the last valid layout while the text has
//! an error), the Enabled toggle and the Apply / Save buttons.
//!
//! Keys: Tab/Shift+Tab and Down/Up move focus Field → Enabled → Apply →
//! Save (wrapping); the field takes printable keys and Left/Right/Home/
//! End/Backspace/Delete; Left/Right/Space toggle Enabled; Enter applies
//! (on Save it saves); Esc cancels. A commit with an error keeps the
//! dialog open. A left click on Enabled/Apply/Save focuses and activates
//! it (COV-075); a click on the field only focuses it.

use crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::views::multiplex::layout_tree::check_fit;
use crate::views::multiplex::{MuxConfig, MuxLayout};

use super::dialog_button_hits::LastLayout;
use super::mux_config_dialog_field::LayoutField;
use super::mux_config_dialog_render::{row_target, RowTarget};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by `Compositor::remove` AND
/// `Compositor::contains` to address the dialog idempotently (R2).
pub const MUX_CONFIG_DIALOG_ID: &str = "mux-config-dialog";

/// Which control of the dialog has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuxDialogFocus {
    Field,
    Enabled,
    Apply,
    Save,
}

const FOCUS_ORDER: [MuxDialogFocus; 4] = [
    MuxDialogFocus::Field,
    MuxDialogFocus::Enabled,
    MuxDialogFocus::Apply,
    MuxDialogFocus::Save,
];

/// Priority::Foreground modal dialog for configuring the mux layout.
pub struct MuxConfigDialog {
    id: String,
    /// The draft being edited; `draft.layout` is the last VALID layout.
    pub(super) draft: MuxConfig,
    pub(super) field: LayoutField,
    pub(super) focus: MuxDialogFocus,
    /// The exact parse or fit error of the field text, if any.
    pub(super) error: Option<String>,
    /// The mux body the layout must fit (the minimum pane size check).
    fit_area: Rect,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
    /// TUI-112: last-rendered row geometry for left-click hit-testing.
    pub(super) last_layout: LastLayout,
}

impl MuxConfigDialog {
    /// A dialog seeded from the live `config`; layouts must fit
    /// `fit_area` (the live mux body).
    pub fn new(config: MuxConfig, fit_area: Rect) -> Self {
        let field = LayoutField::new(&config.layout.to_string());
        Self {
            id: MUX_CONFIG_DIALOG_ID.to_string(),
            draft: config,
            field,
            focus: MuxDialogFocus::Field,
            error: None,
            fit_area,
            action_tx: None,
            pending_action: None,
            last_layout: LastLayout::new(),
        }
    }

    /// Wire the App's action channel (in addition to `pending_action`).
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// The draft config (its layout is the last valid one).
    pub fn draft(&self) -> &MuxConfig {
        &self.draft
    }

    /// The layout field's text.
    pub fn layout_text(&self) -> String {
        self.field.text()
    }

    /// The exact error shown under the field, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn focus(&self) -> MuxDialogFocus {
        self.focus
    }

    /// Test-only: drain any pending action stashed by `handle_event`.
    pub fn take_pending_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    /// TUI-112: the last-rendered row geometry (`None` before render).
    pub fn last_layout(&self) -> &LastLayout {
        &self.last_layout
    }

    fn emit_action(&mut self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            let _ = tx.send(action.clone());
        }
        self.pending_action = Some(action);
    }

    fn remove_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }

    fn move_focus(&mut self, forward: bool) {
        let at = FOCUS_ORDER
            .iter()
            .position(|f| *f == self.focus)
            .unwrap_or(0);
        let n = FOCUS_ORDER.len();
        self.focus = FOCUS_ORDER[if forward {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }];
    }

    /// Re-validate the field text: a valid, fitting layout becomes the
    /// draft layout (the preview follows); otherwise the error shows.
    fn revalidate(&mut self) {
        match MuxLayout::parse(&self.field.text()) {
            Ok(layout) => match check_fit(self.fit_area, &layout) {
                Ok(()) => {
                    self.draft.layout = layout;
                    self.error = None;
                }
                Err(err) => self.error = Some(err.to_string()),
            },
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    fn commit(&mut self, save: bool) -> EventResult {
        if self.error.is_some() {
            return EventResult::consumed();
        }
        let draft = self.draft.clone();
        self.emit_action(if save {
            Action::MuxConfigAppliedAndSaved(draft)
        } else {
            Action::MuxConfigApplied(draft)
        });
        EventResult::Consumed(Some(self.remove_callback()))
    }

    fn activate(&mut self) -> EventResult {
        self.commit(self.focus == MuxDialogFocus::Save)
    }

    fn handle_key(&mut self, key: &KeyEvent) -> EventResult {
        match key.code {
            KeyCode::Esc => return EventResult::Consumed(Some(self.remove_callback())),
            KeyCode::Enter if key.modifiers.is_empty() => return self.activate(),
            KeyCode::Tab | KeyCode::Down => self.move_focus(true),
            KeyCode::BackTab | KeyCode::Up => self.move_focus(false),
            _ if self.focus == MuxDialogFocus::Field => match self.field.handle_key(key) {
                Some(true) => self.revalidate(),
                Some(false) => {}
                None => return EventResult::ignored(),
            },
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                if self.focus == MuxDialogFocus::Enabled =>
            {
                self.draft.enabled = !self.draft.enabled;
            }
            _ => return EventResult::ignored(),
        }
        EventResult::consumed()
    }

    fn handle_click(&mut self, col: u16, row: u16) -> EventResult {
        if !self.last_layout.contains(col, row) {
            return EventResult::ignored();
        }
        match self.last_layout.hit(col, row).and_then(row_target) {
            Some(RowTarget::Field) => {
                self.focus = MuxDialogFocus::Field;
                EventResult::consumed()
            }
            Some(RowTarget::Control(focus)) => {
                self.focus = focus;
                self.activate()
            }
            None => EventResult::consumed(),
        }
    }
}

impl Component for MuxConfigDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) => self.handle_click(m.column, m.row),
                MouseEventKind::ScrollUp => {
                    self.move_focus(false);
                    EventResult::consumed()
                }
                MouseEventKind::ScrollDown => {
                    self.move_focus(true);
                    EventResult::consumed()
                }
                _ => EventResult::ignored(),
            },
            _ => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render_dialog(area, buf);
    }
}
