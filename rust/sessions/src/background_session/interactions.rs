use super::*;
use codelet_rpc_types::{RequestHolder, RequestKind, RequestRoute, RequestStatus};
use codelet_tools::request_user_input::{HitlRequest, HitlResponse};
use codelet_tools::tool_pause::PauseKind;

/// AMGR-020: a raised question or approval with the routing fixed when it
/// was raised: its kind, its chain (origin, its parent, ... the root) and
/// its holder (`None` is the user).
#[derive(Clone)]
pub(crate) struct Raised<T> {
    pub(super) payload: T,
    pub(super) kind: RequestKind,
    pub(super) chain: Vec<Uuid>,
    pub(super) holder: Option<Uuid>,
}

fn session_id(id: Uuid) -> codelet_rpc_types::SessionId {
    codelet_rpc_types::SessionId::from(id.to_string())
}

impl<T> Raised<T> {
    pub(super) fn route(&self, request_id: String) -> RequestRoute {
        RequestRoute {
            request_id,
            origin: session_id(self.chain[0]),
            kind: self.kind,
            chain: self.chain.iter().copied().map(session_id).collect(),
            holder: match self.holder {
                Some(holder) => RequestHolder::Session(session_id(holder)),
                None => RequestHolder::User,
            },
        }
    }

    /// AMGR-020: the user may always answer (taking the request over); a
    /// session only when it holds the request.
    fn accepts(&self, request_id: &str, responder: &RequestHolder) -> Result<(), String> {
        let RequestHolder::Session(responder) = responder else {
            return Ok(());
        };
        if self
            .holder
            .is_some_and(|holder| holder.to_string() == responder.value)
        {
            return Ok(());
        }
        let holder = match self.holder {
            Some(holder) => format!("session {holder}"),
            None => "the user".to_string(),
        };
        Err(format!(
            "Session {} does not hold request {request_id}; its holder is {holder}",
            responder.value
        ))
    }
}

/// BUG-631: a reply to a pause, named by the reply action that sent it.
/// Each action answers exactly one pause kind as the wire shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReply {
    /// AMGR-023 `pause_human_action`: a human action
    /// (`PauseKind::Continue`), done or cancelled.
    HumanAction(codelet_rpc_types::HumanActionReply),
    /// `pause_confirm`: a two-choice confirm (`PauseKind::Confirm`).
    Confirm { accept: bool },
    /// `pause_triple` and the bridge `pause_response`: a three-choice
    /// approval (`PauseKind::Triple`).
    Triple(codelet_rpc_types::ApprovalChoice),
}

impl PauseReply {
    fn action(self) -> &'static str {
        match self {
            Self::HumanAction(_) => "pause_human_action",
            Self::Confirm { .. } => "pause_confirm",
            Self::Triple(_) => "pause_triple",
        }
    }

    fn answers(self, kind: PauseKind) -> bool {
        matches!(
            (self, kind),
            (Self::HumanAction(_), PauseKind::Continue)
                | (Self::Confirm { .. }, PauseKind::Confirm)
                | (Self::Triple(_), PauseKind::Triple)
        )
    }

    /// What the waiting tool receives.
    fn response(self) -> PauseResponse {
        match self {
            Self::HumanAction(reply) => {
                crate::conversions::human_action_reply_to_pause_response(reply)
            }
            Self::Confirm { accept } => {
                crate::conversions::confirm_accept_to_pause_response(accept)
            }
            Self::Triple(choice) => crate::conversions::approval_choice_to_pause_response(choice),
        }
    }
}

fn pause_kind_name(kind: PauseKind) -> &'static str {
    match kind {
        PauseKind::Continue => "a human action",
        PauseKind::Confirm => "a two-choice confirm",
        PauseKind::Triple => "a three-choice approval",
    }
}

fn kind_name(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Question => "a question",
        RequestKind::Approval => "an approval",
        RequestKind::HumanAction => "a human action",
    }
}

impl BackgroundSession {
    /// AMGR-020: this session, its parent, ... the root, read from the
    /// spawner links (each session's first supervisor) now.
    fn request_chain(&self) -> Vec<Uuid> {
        let mut chain = vec![self.id];
        if let Some(manager) = self.owning_manager() {
            let mut current = self.id;
            while let Some(&parent) = manager.get_supervisors(current).first() {
                if chain.contains(&parent) {
                    break;
                }
                chain.push(parent);
                current = parent;
            }
        }
        chain
    }

    /// AMGR-020: route a new request: the origin's parent holds a question
    /// or approval (the user for a root session); the user always holds a
    /// human action. AMGR-021: the first live session above the origin, so
    /// a link to a session that is gone (BUG-587) leaves it with the user.
    fn raise<T>(&self, payload: T, kind: RequestKind) -> Raised<T> {
        let chain = self.request_chain();
        let holder = match kind {
            RequestKind::HumanAction => None,
            RequestKind::Question | RequestKind::Approval => self.first_live_above(&chain, self.id),
        };
        Raised {
            payload,
            kind,
            chain,
            holder,
        }
    }

    /// AMGR-020: tell every session on the request's chain what happened.
    pub(super) fn announce(&self, route: RequestRoute, status: RequestStatus) {
        let manager = self.owning_manager();
        let chain = route.chain.clone();
        let chunk = StreamChunk::InteractionRequestChanged { route, status };
        for member in chain {
            if member.value == self.id.to_string() {
                self.handle_output(chunk.clone());
            } else if let Some(session) = manager
                .as_ref()
                .and_then(|manager| manager.get_session(&member.value).ok())
            {
                session.handle_output(chunk.clone());
            }
        }
    }

    fn announce_replaced<T>(&self, replaced: Option<(String, Raised<T>)>) {
        if let Some((id, raised)) = replaced {
            self.announce(raised.route(id), RequestStatus::Cancelled);
        }
    }

    /// AMGR-021: tell the new request's holder (before announcing it, so
    /// the event a holder in await_idle reacts to comes after the decision),
    /// then tell the chain.
    fn announce_pending<T: Clone + super::request_routing::Asks>(
        &self,
        slot: &crate::interaction_slot::InteractionSlot<Raised<T>>,
    ) {
        if let Ok((id, Some(raised))) = slot.snapshot() {
            self.tell_holder(&id, &raised);
            self.announce(raised.route(id), RequestStatus::Pending);
        }
    }

    pub fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.current().map(|raised| raised.payload)
    }

    /// AMGR-020: the pending approval or human action with its route.
    pub fn pending_pause(&self) -> Option<(RequestRoute, PauseState)> {
        let (id, raised) = self.pause_state.snapshot().ok()?;
        raised.map(|raised| (raised.route(id), raised.payload))
    }

    pub fn pending_pause_route(&self) -> Option<RequestRoute> {
        self.pending_pause().map(|(route, _)| route)
    }

    pub fn set_pause_state(&self, state: Option<PauseState>) {
        let is_paused = state.is_some();
        let raised = state.map(|state| {
            let kind = match state.kind {
                codelet_tools::tool_pause::PauseKind::Continue => RequestKind::HumanAction,
                codelet_tools::tool_pause::PauseKind::Confirm
                | codelet_tools::tool_pause::PauseKind::Triple => RequestKind::Approval,
            };
            self.raise(state, kind)
        });
        match self.pause_state.set(raised) {
            Ok(replaced) => self.announce_replaced(replaced),
            Err(error) => {
                tracing::error!(%error, "Cannot update pause state");
                return;
            }
        }
        if is_paused {
            self.announce_pending(&self.pause_state);
            self.set_status(SessionStatus::Paused);
        }
    }

    pub fn clear_pause_state(&self) {
        match self.pause_state.set(None) {
            Ok(replaced) => self.announce_replaced(replaced),
            Err(error) => {
                tracing::error!(%error, "Cannot clear pause state");
                return;
            }
        }
        self.set_status(SessionStatus::Running);
    }

    fn blocking_recv_compat<T>(recv: impl FnOnce() -> T) -> T {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(recv)
            }
            _ => recv(),
        }
    }

    pub fn wait_for_pause_response(&self) -> PauseResponse {
        self.cancel_if_interrupted();
        Self::blocking_recv_compat(|| {
            let rx = self
                .pause_response_rx
                .lock()
                .expect("pause_response_rx lock poisoned");
            rx.recv().unwrap_or(PauseResponse::Interrupted)
        })
    }

    /// AMGR-020: answer the pending approval or human action named
    /// `request_id`, from the user or its holder session. BUG-631: `reply`
    /// must be the action for the pause's kind; any other answers nothing.
    pub fn answer_pause(
        &self,
        request_id: &str,
        responder: &RequestHolder,
        reply: PauseReply,
    ) -> Result<(), String> {
        if let Some(question) = self.hitl_request.pending_with_id(request_id) {
            return Err(format!(
                "Request {request_id} is {}, not an approval",
                kind_name(question.kind)
            ));
        }
        let answered = self.pause_state.respond(
            request_id,
            |raised| {
                if !reply.answers(raised.payload.kind) {
                    return Err(format!(
                        "Request {request_id} is {}; {} does not answer it",
                        pause_kind_name(raised.payload.kind),
                        reply.action()
                    ));
                }
                raised.accepts(request_id, responder)
            },
            || {
                self.pause_response_tx
                    .send(reply.response())
                    .map_err(|_| "Pause response channel closed".into())
            },
        )?;
        self.set_status(SessionStatus::Running);
        if *responder == RequestHolder::User {
            self.tell_taken_over(
                request_id,
                &answered,
                super::request_routing::paused_with(reply),
            );
        }
        self.announce(
            answered.route(request_id.to_string()),
            RequestStatus::Answered {
                by: responder.clone(),
            },
        );
        Ok(())
    }

    /// AMGR-020: answer the pending question named `request_id`, from the
    /// user or its holder session.
    pub fn answer_question(
        &self,
        request_id: &str,
        responder: &RequestHolder,
        response: HitlResponse,
    ) -> Result<(), String> {
        if let Some(pause) = self.pause_state.pending_with_id(request_id) {
            return Err(format!(
                "Request {request_id} is {}, not a question",
                kind_name(pause.kind)
            ));
        }
        let how = super::request_routing::answered_with(&response);
        let answered = self.hitl_request.respond(
            request_id,
            |raised| raised.accepts(request_id, responder),
            || {
                self.hitl_response_tx
                    .send(response)
                    .map_err(|_| "HITL response channel closed".into())
            },
        )?;
        if *responder == RequestHolder::User {
            self.tell_taken_over(request_id, &answered, &how);
        }
        self.announce(
            answered.route(request_id.to_string()),
            RequestStatus::Answered {
                by: responder.clone(),
            },
        );
        Ok(())
    }

    /// BUG-605: answer the question and the approval this session waits on
    /// with their cancelled outcomes ({cancelled: true} and Interrupted).
    /// Every interrupt calls this, so a closed or interrupted session's
    /// wait always returns. AMGR-020: the chain is told they were cancelled.
    pub fn cancel_pending_interactions(&self) {
        match self.pause_state.cancel_pending(|| {
            self.pause_response_tx
                .send(PauseResponse::Interrupted)
                .map_err(|_| "Pause response channel closed".into())
        }) {
            Ok(cancelled) => self.announce_replaced(cancelled),
            Err(error) => tracing::error!(%error, "Cannot cancel the pending approval"),
        }
        match self.hitl_request.cancel_pending(|| {
            self.hitl_response_tx
                .send(HitlResponse::Cancelled { cancelled: true })
                .map_err(|_| "HITL response channel closed".into())
        }) {
            Ok(cancelled) => self.announce_replaced(cancelled),
            Err(error) => tracing::error!(%error, "Cannot cancel the pending question"),
        }
    }

    /// BUG-605: a request stored after the interrupt flag was set is
    /// answered before waiting. The interrupt sets the flag before it
    /// cancels under the slot lock, so either it sees this request or this
    /// check sees its flag.
    fn cancel_if_interrupted(&self) {
        if self.is_interrupted.load(Ordering::Acquire) {
            self.cancel_pending_interactions();
        }
    }

    /// BUG-605: the turn resumes after an answered wait only if the session
    /// is still paused on it; a wait answered after its turn ended leaves
    /// the status the turn set.
    pub fn resume_after_interaction(&self) {
        let paused = SessionStatus::Paused as u8;
        let running = SessionStatus::Running as u8;
        let _order = self.status_order();
        if self
            .status
            .compare_exchange(paused, running, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.notify_status_change(paused, SessionStatus::Running);
        }
    }

    pub fn wait_for_fspec_response(&self) -> FspecResult {
        Self::blocking_recv_compat(|| {
            let rx = self
                .fspec_response_rx
                .lock()
                .expect("fspec_response_rx lock poisoned");
            rx.recv().unwrap_or_else(|_| FspecResult {
                success: false,
                data: String::new(),
                error: Some("Fspec response channel closed unexpectedly".to_string()),
                system_reminder: None,
                tool_call_id: String::new(),
            })
        })
    }

    pub fn send_fspec_result(&self, result: FspecResult) {
        if let Err(e) = self.fspec_response_tx.send(result) {
            tracing::error!(error = %e, "[FSPEC_SESSION] Failed to send fspec result");
        }
    }

    pub fn wait_for_hitl_response(&self) -> HitlResponse {
        self.cancel_if_interrupted();
        Self::blocking_recv_compat(|| {
            let rx = self
                .hitl_response_rx
                .lock()
                .expect("hitl_response_rx lock poisoned");
            rx.recv()
                .unwrap_or(HitlResponse::Cancelled { cancelled: true })
        })
    }

    pub fn set_hitl_request(&self, request: Option<HitlRequest>) {
        let is_asking = request.is_some();
        let raised = request.map(|request| self.raise(request, RequestKind::Question));
        match self.hitl_request.set(raised) {
            Ok(replaced) => self.announce_replaced(replaced),
            Err(error) => {
                tracing::error!(%error, "Cannot update HITL request");
                return;
            }
        }
        if is_asking {
            self.announce_pending(&self.hitl_request);
        }
    }

    pub fn get_hitl_request(&self) -> Option<HitlRequest> {
        self.hitl_request.current().map(|raised| raised.payload)
    }

    /// AMGR-020: the pending question with its route.
    pub fn pending_question(&self) -> Option<(RequestRoute, HitlRequest)> {
        let (id, raised) = self.hitl_request.snapshot().ok()?;
        raised.map(|raised| (raised.route(id), raised.payload))
    }

    pub fn pending_question_route(&self) -> Option<RequestRoute> {
        self.pending_question().map(|(route, _)| route)
    }
}
