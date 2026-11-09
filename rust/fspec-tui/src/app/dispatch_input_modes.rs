//! RIG-016: input submitted while the focused session's turn is running is
//! steered (Enter) or queued (Alt+Enter) instead of sent as a new turn.

use codelet_rpc_types::{InputMode, SessionId, SessionStatus};

use super::state::App;
use crate::components::Action;

impl App {
    /// TUI-115: the mode Enter submits under while `session` is busy —
    /// a running turn is steered, a compaction is queued behind (steering
    /// into compaction makes no sense). `None` while the session is idle.
    pub(crate) fn busy_enter_mode(&self, session: &SessionId) -> Option<InputMode> {
        match self.agent_view_store.session_status_for(session) {
            Some(SessionStatus::Running) => Some(InputMode::Steer),
            Some(SessionStatus::Compacting) => Some(InputMode::Queue),
            _ => None,
        }
    }

    /// TUI-115: record a busy submission for its transcript tag and the
    /// queued list, then submit it to the session under `mode`.
    pub(crate) fn submit_busy_input(&mut self, session: SessionId, text: String, mode: InputMode) {
        match self.agent_view_store.session_context_mut_for(&session) {
            Some(context) => match mode {
                InputMode::Steer => context.record_steer(text.clone()),
                InputMode::Queue => {
                    context.record_queued(text.clone());
                }
                // An interrupting input starts as a plain new turn.
                InputMode::Interrupt => {}
            },
            None => tracing::warn!(
                session = %session.value,
                "[TUI-115] no agent-view context to tag a {mode:?} input"
            ),
        }
        self.spawn_submit_input(session, text, mode);
    }

    /// Spawn `backend.submit_input` for `session` under `mode`, parked so
    /// `drain_pending` harnesses await it (RPC-012 test seam).
    pub(crate) fn spawn_submit_input(&mut self, session: SessionId, text: String, mode: InputMode) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Err(error) = backend.submit_input(session.clone(), text, mode).await {
                tracing::warn!("[RIG-016] submit_input ({mode:?}) failed: {error}");
                // BUG-328: a refused input is shown in the session.
                let _ = action_tx.send(Action::EmitSessionNotice(
                    session,
                    format!("[error] send failed: {error}"),
                ));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// `Action::InputQueued`: queue the text behind `session`'s running
    /// turn — BUG-552: the session it was typed in, never the focus at
    /// handling time.
    pub(crate) fn handle_input_queued(&mut self, session: SessionId, text: String) {
        self.end_composer_draft(&session); // BUG-549 rule 2
        self.submit_busy_input(session.clone(), text.clone(), InputMode::Queue);
        self.handle_input_submitted_persistence(session, text);
    }
}
