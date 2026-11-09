//! AMGR-021: who holds a request, and telling a session that it does.
//!
//! A request is held by the first live session above its origin on its
//! chain (the user when there is none; the user always holds a human
//! action). A session that comes to hold one (raised, escalated or passed
//! up to it) is told before the request is announced: one inside
//! AgentManager await_idle on a session of the chain learns it from the
//! `InteractionRequestChanged` event; any other gets a message that starts
//! its next turn. A holder that is interrupted or closed passes what it
//! holds up the chain, and a holder whose request the user answered is told.

use super::interactions::Raised;
use super::*;
use codelet_rpc_types::{RequestKind, RequestRoute, RequestStatus};
use codelet_tools::request_user_input::{HitlRequest, HitlResponse};

/// AMGR-021: a pending question or approval with its route, as AgentManager
/// reports it.
#[derive(Debug, Clone)]
pub enum PendingInteraction {
    Question(RequestRoute, HitlRequest),
    Pause(RequestRoute, PauseState),
}

impl PendingInteraction {
    pub fn route(&self) -> &RequestRoute {
        match self {
            Self::Question(route, _) | Self::Pause(route, _) => route,
        }
    }
}

/// AMGR-021: what a raised request asks, in the words a holder is told.
pub(super) trait Asks {
    fn asks(&self) -> String;
}

impl Asks for HitlRequest {
    fn asks(&self) -> String {
        self.questions
            .iter()
            .map(|question| {
                let options = question
                    .options
                    .as_ref()
                    .map(|options| {
                        let labels: Vec<&str> =
                            options.iter().map(|option| option.label.as_str()).collect();
                        format!(" (options: {})", labels.join(", "))
                    })
                    .unwrap_or_default();
                format!("- [{}] {}{options}", question.id, question.question)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Asks for PauseState {
    fn asks(&self) -> String {
        let details = self
            .details
            .as_deref()
            .map(|details| format!("\n{details}"))
            .unwrap_or_default();
        format!("{}: {}{details}", self.tool_name, self.message)
    }
}

/// The words for an answer the user gave, in the message to the holder it
/// took the request from.
pub(super) fn answered_with(response: &HitlResponse) -> String {
    match response {
        HitlResponse::Cancelled { .. } => "cancelled it".to_string(),
        HitlResponse::Answered { answers } => {
            let mut parts: Vec<String> = answers
                .iter()
                .map(|(id, answer)| {
                    let mut chosen = answer.selected.clone();
                    chosen.extend(answer.other.clone());
                    format!("{id}: {}", chosen.join(", "))
                })
                .collect();
            parts.sort();
            format!("answered {}", parts.join("; "))
        }
    }
}

pub(super) fn paused_with(reply: super::PauseReply) -> &'static str {
    use codelet_rpc_types::ApprovalChoice;
    match reply {
        super::PauseReply::HumanAction(codelet_rpc_types::HumanActionReply::Done) => "did it",
        super::PauseReply::HumanAction(codelet_rpc_types::HumanActionReply::Cancel) => {
            "cancelled it"
        }
        super::PauseReply::Confirm { accept: true } => "approved it",
        super::PauseReply::Confirm { accept: false } => "denied it",
        super::PauseReply::Triple(ApprovalChoice::Approve) => "approved it once",
        super::PauseReply::Triple(ApprovalChoice::ApproveSession) => "approved it for the session",
        super::PauseReply::Triple(ApprovalChoice::Deny) => "denied it",
    }
}

/// AMGR-021: an AgentManager await_idle in progress; dropping it ends it.
pub struct AwaitRegistration {
    session: Arc<BackgroundSession>,
    awaited: Vec<Uuid>,
}

impl Drop for AwaitRegistration {
    fn drop(&mut self) {
        if let Ok(mut awaited) = self.session.awaited.lock() {
            for id in &self.awaited {
                if let Some(index) = awaited.iter().position(|awaiting| awaiting == id) {
                    awaited.swap_remove(index);
                }
            }
        }
    }
}

impl BackgroundSession {
    /// AMGR-021: record that this session is inside await_idle on
    /// `sessions` until the registration is dropped.
    pub fn begin_await(self: &Arc<Self>, sessions: &[Uuid]) -> AwaitRegistration {
        if let Ok(mut awaited) = self.awaited.lock() {
            awaited.extend_from_slice(sessions);
        }
        AwaitRegistration {
            session: self.clone(),
            awaited: sessions.to_vec(),
        }
    }

    /// True while this session awaits a session of `chain`.
    fn awaits_any(&self, chain: &[Uuid]) -> bool {
        self.awaited
            .lock()
            .map(|awaited| awaited.iter().any(|id| chain.contains(id)))
            .unwrap_or(false)
    }

    /// AMGR-021: true the first time await_idle reports the human action
    /// `request_id` to this session; a parent is told once.
    pub fn first_report_of(&self, request_id: &str) -> bool {
        self.reported_requests
            .lock()
            .map(|mut reported| reported.insert(request_id.to_string()))
            .unwrap_or(false)
    }

    /// AMGR-021: the first live session above `from` on `chain`, if any
    /// (a member that was closed, or whose link outlived a daemon restart,
    /// is skipped: BUG-587).
    pub(super) fn first_live_above(&self, chain: &[Uuid], from: Uuid) -> Option<Uuid> {
        let manager = self.owning_manager()?;
        let start = chain.iter().position(|id| *id == from)? + 1;
        chain[start..]
            .iter()
            .copied()
            .find(|id| manager.get_session(&id.to_string()).is_ok())
    }

    fn live_session(&self, id: Uuid) -> Option<Arc<BackgroundSession>> {
        if id == self.id {
            return None;
        }
        self.owning_manager()?.get_session(&id.to_string()).ok()
    }

    fn display_name(&self) -> String {
        let name = self
            .name
            .read()
            .map(|name| name.clone())
            .unwrap_or_default();
        if name.is_empty() {
            format!("session {}", self.id)
        } else {
            format!("\"{name}\" ({})", self.id)
        }
    }

    /// Queue `message` from this (origin) session for `session`: it starts
    /// that session's next turn, at once if it is idle.
    fn tell(&self, session: &BackgroundSession, message: String) {
        let input = IncomingMessage {
            source_session_id: self.id.to_string(),
            role_name: format!("sub-agent request from {}", self.display_name()),
            message,
            images: None,
            interrupt_epoch: None,
            submission_id: None,
        };
        if let Err(error) = session.receive_incoming_message(input) {
            tracing::error!(%error, holder = %session.id, "Cannot tell a session about a request");
        }
    }

    /// AMGR-021: tell whoever must now act on the request `raised` (named
    /// `request_id`) that it waits on them, unless they are inside
    /// await_idle on a session of its chain (the event tells them then).
    pub(super) fn tell_holder<T: Asks>(&self, request_id: &str, raised: &Raised<T>) {
        let asks = raised.payload.asks();
        let (target, message) = match (raised.kind, raised.holder) {
            (RequestKind::HumanAction, _) => {
                let Some(parent) = self.first_live_above(&raised.chain, self.id) else {
                    return;
                };
                (
                    parent,
                    format!(
                        "Your sub-agent {} waits on the user (request {request_id}): {asks}\n\
                         Only the user can do this; it is shown to the user and you cannot answer it.",
                        self.display_name()
                    ),
                )
            }
            (RequestKind::Question, Some(holder)) => (
                holder,
                format!(
                    "Your sub-agent {} asks you (request {request_id}):\n{asks}\n\
                     Reply with AgentManager answer {{\"request_id\": \"{request_id}\", \"answers\": \
                     [{{\"id\": \"<question id>\", \"selected\": [\"<option>\"], \"other\": \"<text>\"}}]}} \
                     (or \"cancelled\": true), or pass it up to your own parent or the user with \
                     AgentManager escalate {{\"request_id\": \"{request_id}\"}}. Do not message the \
                     sub-agent: it is paused on this request.",
                    self.display_name()
                ),
            ),
            (RequestKind::Approval, Some(holder)) => (
                holder,
                format!(
                    "Your sub-agent {} needs your approval (request {request_id}):\n{asks}\n\
                     Decide with AgentManager approve {{\"request_id\": \"{request_id}\", \"choice\": \
                     \"approve\" | \"approve_session\" | \"deny\"}}, or pass it up to your own parent \
                     or the user with AgentManager escalate {{\"request_id\": \"{request_id}\"}}. Do not \
                     message the sub-agent: it is paused on this request.",
                    self.display_name()
                ),
            ),
            (_, None) => return,
        };
        let Some(session) = self.live_session(target) else {
            return;
        };
        if !session.awaits_any(&raised.chain) {
            self.tell(&session, message);
        }
    }

    /// AMGR-021: tell the session the user took request `request_id` over
    /// from that it was answered, and how.
    pub(super) fn tell_taken_over<T>(&self, request_id: &str, raised: &Raised<T>, how: &str) {
        let Some(session) = raised.holder.and_then(|holder| self.live_session(holder)) else {
            return;
        };
        self.tell(
            &session,
            format!(
                "The user answered request {request_id} of your sub-agent {} for you: the user {how}. \
                 You no longer hold it.",
                self.display_name()
            ),
        );
    }

    /// AMGR-021: move the request named `request_id`, held by `caller`, to
    /// the next live session above the caller on its chain, or the user.
    pub fn escalate_request(&self, request_id: &str, caller: Uuid) -> Result<RequestRoute, String> {
        if let Some(route) = self.move_question(request_id, |raised| {
            check_escalation(request_id, raised, caller)
        })? {
            return Ok(route);
        }
        self.move_pause(request_id, |raised| {
            check_escalation(request_id, raised, caller)
        })?
        .ok_or_else(|| "Stale or already answered interaction".to_string())
    }

    /// AMGR-021: this holder was interrupted or closed: every request it
    /// holds moves to the next live session above it, or the user.
    pub(super) fn pass_up_held_requests(&self) {
        let Some(manager) = self.owning_manager() else {
            return;
        };
        let holder = self.id;
        for id in manager.all_in_memory_session_ids() {
            let Ok(origin) = manager.get_session(&id) else {
                continue;
            };
            let moves = |raised: &Raised<_>| (raised.holder == Some(holder)).then_some(holder);
            if let Err(error) = origin.move_pending_question(moves) {
                tracing::error!(%error, "Cannot pass a question up its chain");
            }
            let moves = |raised: &Raised<_>| (raised.holder == Some(holder)).then_some(holder);
            if let Err(error) = origin.move_pending_pause(moves) {
                tracing::error!(%error, "Cannot pass an approval up its chain");
            }
        }
    }

    fn move_question(
        &self,
        request_id: &str,
        check: impl FnOnce(&Raised<HitlRequest>) -> Result<Uuid, String>,
    ) -> Result<Option<RequestRoute>, String> {
        let mut check = Some(check);
        self.move_in(&self.hitl_request, |id, raised| {
            if id != request_id {
                return Ok(None);
            }
            check.take().map(|check| check(raised)).transpose()
        })
    }

    fn move_pause(
        &self,
        request_id: &str,
        check: impl FnOnce(&Raised<PauseState>) -> Result<Uuid, String>,
    ) -> Result<Option<RequestRoute>, String> {
        let mut check = Some(check);
        self.move_in(&self.pause_state, |id, raised| {
            if id != request_id {
                return Ok(None);
            }
            check.take().map(|check| check(raised)).transpose()
        })
    }

    fn move_pending_question(
        &self,
        moves: impl FnOnce(&Raised<HitlRequest>) -> Option<Uuid>,
    ) -> Result<Option<RequestRoute>, String> {
        let mut moves = Some(moves);
        self.move_in(&self.hitl_request, |_, raised| {
            Ok(moves.take().and_then(|moves| moves(raised)))
        })
    }

    fn move_pending_pause(
        &self,
        moves: impl FnOnce(&Raised<PauseState>) -> Option<Uuid>,
    ) -> Result<Option<RequestRoute>, String> {
        let mut moves = Some(moves);
        self.move_in(&self.pause_state, |_, raised| {
            Ok(moves.take().and_then(|moves| moves(raised)))
        })
    }

    /// Move the pending request of `slot` when `from` names the session it
    /// moves up from: its holder becomes the first live session above that
    /// one (the user if none); the new holder is told and the chain is told.
    fn move_in<T: Clone + Asks>(
        &self,
        slot: &crate::interaction_slot::InteractionSlot<Raised<T>>,
        mut from: impl FnMut(&str, &Raised<T>) -> Result<Option<Uuid>, String>,
    ) -> Result<Option<RequestRoute>, String> {
        let moved = slot.modify(|id, raised| {
            let Some(above) = from(id, raised)? else {
                return Ok(None);
            };
            raised.holder = self.first_live_above(&raised.chain, above);
            Ok(Some((id.to_string(), raised.clone())))
        })?;
        let Some((id, raised)) = moved.flatten() else {
            return Ok(None);
        };
        self.tell_holder(&id, &raised);
        let route = raised.route(id);
        self.announce(route.clone(), RequestStatus::Pending);
        Ok(Some(route))
    }

    /// AMGR-021: this session's pending question and approval with their
    /// routes.
    pub fn pending_interactions(&self) -> Vec<PendingInteraction> {
        let mut pending = Vec::new();
        if let Some((route, request)) = self.pending_question() {
            pending.push(PendingInteraction::Question(route, request));
        }
        if let Some((route, state)) = self.pending_pause() {
            pending.push(PendingInteraction::Pause(route, state));
        }
        pending
    }
}

/// AMGR-021: `caller` may escalate a question or approval it holds; a human
/// action is the user's alone.
fn check_escalation<T>(request_id: &str, raised: &Raised<T>, caller: Uuid) -> Result<Uuid, String> {
    if raised.kind == RequestKind::HumanAction {
        return Err(format!(
            "Request {request_id} is a human action; only the user can do it"
        ));
    }
    if raised.holder != Some(caller) {
        let holder = match raised.holder {
            Some(holder) => format!("session {holder}"),
            None => "the user".to_string(),
        };
        return Err(format!(
            "Session {caller} does not hold request {request_id}; its holder is {holder}"
        ));
    }
    Ok(caller)
}
