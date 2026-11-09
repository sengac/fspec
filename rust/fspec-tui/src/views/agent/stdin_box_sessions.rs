//! TUI-117 — per-session stdin answer box state.
//!
//! Feature: spec/features/stdin-prompt-recovery-and-per-session-state.feature
//!
//! Every agent session owns its answer box: the input buffer, whether it has
//! focus, the red error row of a failed send, and the history of unsent text
//! (Shift+Up / Shift+Down). The view resolves the box through the session of
//! the painted request (`last_exec_stdin`), so switching sessions shows the
//! incoming session's own box with no swap step.

use std::collections::HashMap;

use codelet_rpc_types::{ExecStdinControl, SessionId};

use crossterm::event::Event;

use super::multiline_input::{InputGate, MultiLineInput};
use super::AgentView;

/// One session's answer box.
#[derive(Default)]
pub struct SessionStdinBox {
    pub(super) input: MultiLineInput,
    focused: bool,
    error: Option<String>,
    history: Vec<String>,
    /// `(history index, text in the box when recall started)` while
    /// Shift+Up / Shift+Down walk the history.
    recall: Option<(usize, String)>,
}

impl SessionStdinBox {
    /// The red error row of the last failed send, if any.
    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// An edit happened: the error row and any history walk end.
    pub(super) fn edited(&mut self) {
        self.error = None;
        self.recall = None;
    }

    /// Feed a key or paste to the input; a change to the text ends the
    /// error row and any history walk. Plain Enter never reaches here, so
    /// the outcome is only ever Continued or Ignored.
    pub(super) fn apply_event(&mut self, event: &Event) {
        let before = self.input.value();
        self.input.handle_event_gated(event, InputGate::default());
        if self.input.value() != before {
            self.edited();
        }
    }

    /// Shift+Up: show the next older history entry.
    pub(super) fn recall_older(&mut self) {
        let older = match &self.recall {
            None => self.history.len().checked_sub(1),
            Some((index, _)) => index.checked_sub(1),
        };
        let Some(index) = older else {
            return;
        };
        let Some(text) = self.history.get(index).cloned() else {
            return;
        };
        let stash = match self.recall.take() {
            Some((_, stash)) => stash,
            None => self.input.value(),
        };
        self.recall = Some((index, stash));
        self.input.set_value(&text);
    }

    /// Shift+Down: show the next newer entry, or the text the walk started
    /// from once past the newest.
    pub(super) fn recall_newer(&mut self) {
        let Some((index, stash)) = self.recall.take() else {
            return;
        };
        let newer = index
            .checked_add(1)
            .and_then(|next| self.history.get(next).cloned().map(|text| (next, text)));
        match newer {
            Some((newer, text)) => {
                self.input.set_value(&text);
                self.recall = Some((newer, stash));
            }
            None => self.input.set_value(&stash),
        }
    }

    /// Move non-empty `text` into the history.
    fn remember(&mut self, text: String) {
        if !text.trim().is_empty() {
            self.history.push(text);
        }
    }

    /// The request closed: unfocus and keep unsent text in the history.
    fn close(&mut self) {
        let unsent = self.input.value();
        self.input.set_value("");
        self.remember(unsent);
        self.focused = false;
        self.error = None;
        self.recall = None;
    }

    /// A send failed: put `text` back (text typed since goes to the
    /// history) and show `error`.
    fn restore_failed(&mut self, text: &str, error: String) {
        let typed_since = self.input.value();
        self.remember(typed_since);
        self.input.set_value(text);
        self.recall = None;
        self.error = Some(error);
    }
}

/// The answer boxes of every session, keyed by agent session.
#[derive(Default)]
pub struct StdinAnswerBox {
    sessions: HashMap<SessionId, SessionStdinBox>,
}

impl StdinAnswerBox {
    /// True while `session`'s box holds focus.
    pub fn is_focused_for(&self, session: &SessionId) -> bool {
        self.sessions
            .get(session)
            .is_some_and(|state| state.focused)
    }

    /// `session`'s box, if it was ever opened.
    pub(super) fn get(&self, session: &SessionId) -> Option<&SessionStdinBox> {
        self.sessions.get(session)
    }

    /// `session`'s box, created empty on first use.
    pub(super) fn session_mut(&mut self, session: &SessionId) -> &mut SessionStdinBox {
        self.sessions.entry(session.clone()).or_default()
    }

    pub(super) fn focus(&mut self, session: &SessionId) {
        self.session_mut(session).focused = true;
    }

    pub(super) fn unfocus(&mut self, session: &SessionId) {
        self.session_mut(session).focused = false;
    }
}

/// The error row text for a failed control.
fn control_failure(control: ExecStdinControl, error: &str) -> String {
    match control {
        ExecStdinControl::Interrupt => format!("Interrupt failed: {error}"),
        ExecStdinControl::Eof => format!("End of input failed: {error}"),
    }
}

impl AgentView {
    /// TUI-117: `session`'s request closed (exit, new output, a re-probe
    /// with no request, a successful send, or Ctrl+X).
    pub(crate) fn close_stdin_box(&mut self, session: &SessionId) {
        self.stdin_box.session_mut(session).close();
    }

    /// TUI-117: a write failed while its request is still open.
    pub(crate) fn stdin_send_failed(&mut self, session: &SessionId, text: &str, error: &str) {
        self.stdin_box
            .session_mut(session)
            .restore_failed(text, format!("Not sent: {error}"));
    }

    /// TUI-117: a write failed after its request closed — keep the text in
    /// the history like any other unsent text.
    pub(crate) fn stdin_send_failed_after_close(&mut self, session: &SessionId, text: String) {
        self.stdin_box.session_mut(session).remember(text);
    }

    /// TUI-117: a control (Ctrl+C / Ctrl+D) failed.
    pub(crate) fn stdin_control_failed(
        &mut self,
        session: &SessionId,
        control: ExecStdinControl,
        error: &str,
    ) {
        self.stdin_box.session_mut(session).error = Some(control_failure(control, error));
    }
}
