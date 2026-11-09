//! BUG-549 — the composer-draft lifecycle: a draft belongs to one session
//! and ends when it is submitted.
//!
//! Feature: spec/features/composer-draft-belongs-to-one-session.feature
//!
//! The AgentView has ONE live composer (`navigator.agent.input`). It always
//! holds the draft of exactly one session, `ComposerDrafts::bound`.
//! [`App::sync_composer_draft`] runs at the end of every `App::dispatch`,
//! `App::handle_event` and `App::handle_paste`:
//!
//!   - focus moved (any route: cycle, mux, board chip, attach, a new or
//!     child session, a closed tab): the composer text becomes the bound
//!     session's draft (dropped when its tab is gone) and the composer then
//!     shows the focused session's in-memory draft, or empty;
//!   - otherwise any change of the composer text (typing, history recall,
//!     search insert, popup fill, a submit or HITL capture that cleared it)
//!     is recorded as the focused session's draft.
//!
//! Recording a draft sets `SessionContext.input_draft` and schedules a
//! daemon save on the session's own [`DraftSlot`]: every write takes the
//! slot's next generation; a non-empty draft is debounced by
//! [`DRAFT_SAVE_DEBOUNCE`], an empty one is written as `None` at once.
//! After its debounce a write takes the slot's ordering lock and writes
//! only while it is still the newest generation, so one session's writes
//! reach the daemon one at a time in order (the last write wins) and no
//! session can cancel another's save.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use codelet_rpc_types::SessionId;
use crossterm::event::Event;
use tokio::task::JoinHandle;

use crate::components::EventResult;

use super::state::App;

/// The RPC-052 keystroke-to-daemon debounce for a non-empty draft.
pub(crate) const DRAFT_SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

/// One session's daemon-save slot.
#[derive(Clone, Default)]
pub(crate) struct DraftSlot {
    /// Generation of the newest scheduled draft change (0: none yet).
    pub(crate) latest: Arc<AtomicU64>,
    /// Newest generation the daemon is known to hold. Below `latest` while
    /// a write is pending, or after a write failed (the local copy is then
    /// the newer one).
    pub(crate) settled: Arc<AtomicU64>,
    /// Held while a write or a hydration read talks to the daemon.
    pub(crate) order: Arc<tokio::sync::Mutex<()>>,
}

impl DraftSlot {
    /// Take the next generation; every write scheduled before it is
    /// superseded.
    fn next_generation(&self) -> u64 {
        self.latest.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The session's tab is gone: supersede every pending write and treat
    /// the daemon copy as settled.
    fn retire(&self) {
        let generation = self.next_generation();
        self.settled.fetch_max(generation, Ordering::SeqCst);
    }
}

/// BUG-549 App state: which session's draft the shared composer holds, and
/// a save slot per session.
#[derive(Default)]
pub(crate) struct ComposerDrafts {
    bound: Option<SessionId>,
    slots: HashMap<SessionId, DraftSlot>,
    /// Debounced saves still sleeping or writing (not parked on
    /// `pending_tasks`, so a test drain never waits out the debounce).
    saves: Vec<JoinHandle<()>>,
}

impl ComposerDrafts {
    /// `session`'s slot, created on first use.
    pub(crate) fn slot(&mut self, session: &SessionId) -> DraftSlot {
        self.slots.entry(session.clone()).or_default().clone()
    }

    pub(crate) fn bound(&self) -> Option<&SessionId> {
        self.bound.as_ref()
    }
}

impl App {
    /// Process one crossterm event ([`Self::route_event`]), then keep the
    /// shared composer bound to the focused session (BUG-549).
    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        // TUI-130: c and Ctrl+C copy the text of the dialog on top.
        if let Some(result) = self.copy_from_dialog(event) {
            return result;
        }
        let result = self.route_event(event);
        self.sync_composer_draft();
        result
    }

    /// Route one paste ([`Self::route_paste`]), then keep the shared
    /// composer bound to the focused session (BUG-549).
    pub fn handle_paste(&mut self, text: &str) -> EventResult {
        let result = self.route_paste(text);
        self.sync_composer_draft();
        result
    }

    /// BUG-549: bind the shared composer to the focused session. See the
    /// module docs.
    pub(crate) fn sync_composer_draft(&mut self) {
        self.retire_closed_draft_slots();
        let focused = self.agent_view_store.current_session().cloned();
        let text = self.navigator.agent.input.value();
        // BUG-552: the composer's submissions are stamped with the session
        // whose draft it holds.
        self.navigator.agent.set_composer_session(focused.clone());
        if self.composer_drafts.bound == focused {
            if let Some(session) = focused {
                self.record_draft(&session, text);
            }
            return;
        }
        let incoming = match self.composer_drafts.bound.take() {
            // The outgoing session keeps the composer text as its draft
            // (a no-op when its tab is gone).
            Some(outgoing) => {
                self.record_draft(&outgoing, text);
                self.in_memory_draft(focused.as_ref())
            }
            // Text typed while no session was open belongs to the first
            // session that takes the composer.
            None if !text.is_empty() => {
                if let Some(session) = &focused {
                    self.record_draft(session, text.clone());
                }
                text
            }
            None => self.in_memory_draft(focused.as_ref()),
        };
        if self.navigator.agent.input.value() != incoming {
            self.navigator.agent.input.set_value(&incoming);
        }
        self.composer_drafts.bound = focused;
        self.should_render = true;
    }

    fn in_memory_draft(&self, session: Option<&SessionId>) -> String {
        session
            .and_then(|session| self.agent_view_store.session_context_for(session))
            .map(|context| context.input_draft.clone())
            .unwrap_or_default()
    }

    /// Record `text` as `session`'s draft: the in-memory copy and a daemon
    /// save. No-op when the text is unchanged or the tab is not open.
    pub(crate) fn record_draft(&mut self, session: &SessionId, text: String) {
        let Some(context) = self.agent_view_store.session_context_mut_for(session) else {
            return;
        };
        if context.input_draft == text {
            return;
        }
        context.input_draft.clone_from(&text);
        self.schedule_draft_save(session, text);
    }

    /// BUG-549 rule 2/6: `session`'s draft ends (submitted, cleared): the
    /// in-memory copy empties and `None` is written at once, superseding any
    /// save still pending. The composer is left alone — a submit already
    /// emptied it, and text typed since is a new draft.
    pub(crate) fn end_composer_draft(&mut self, session: &SessionId) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.input_draft.clear();
        }
        self.schedule_draft_save(session, String::new());
    }

    /// Schedule a daemon write of `text` (empty: `None`) on `session`'s
    /// slot. A non-empty draft waits out the debounce first.
    pub(crate) fn schedule_draft_save(&mut self, session: &SessionId, text: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let slot = self.composer_drafts.slot(session);
        let generation = slot.next_generation();
        let (value, debounce) = if text.is_empty() {
            (None, None)
        } else {
            (Some(text), Some(DRAFT_SAVE_DEBOUNCE))
        };
        let backend = self.backend.clone();
        let id = session.clone();
        let handle = tokio::spawn(async move {
            if let Some(delay) = debounce {
                tokio::time::sleep(delay).await;
            }
            let _order = slot.order.lock().await;
            if slot.latest.load(Ordering::SeqCst) != generation {
                return; // superseded by a newer draft change
            }
            match backend.set_pending_input(id.clone(), value).await {
                Ok(()) => {
                    slot.settled.fetch_max(generation, Ordering::SeqCst);
                }
                Err(error) => tracing::debug!(
                    target: "rpc052",
                    session = &id.value,
                    %error,
                    "set_pending_input failed (the local draft stays the newer copy)",
                ),
            }
        });
        if debounce.is_some() {
            self.composer_drafts
                .saves
                .retain(|save| !save.is_finished());
            self.composer_drafts.saves.push(handle);
        } else {
            // RPC-012 drain seam: an immediate write is awaited by
            // `drain_pending` harnesses like the other backend calls.
            self.pending_tasks.push(handle);
        }
    }

    /// BUG-549 rule 6: a session whose tab is gone (closed, deleted, removed
    /// by another client, merged) never receives a late save.
    fn retire_closed_draft_slots(&mut self) {
        let open = self.agent_view_store.open_sessions();
        for (session, slot) in &self.composer_drafts.slots {
            if !open.iter().any(|context| &context.id == session) {
                slot.retire();
            }
        }
    }
}
