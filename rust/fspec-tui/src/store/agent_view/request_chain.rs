//! AMGR-022 — the sub-agent requests pending on each chain, as the TUI
//! learns them from `StreamChunk::InteractionRequestChanged`.
//!
//! Feature: spec/features/a-sub-agents-request-shows-in-the-root-view.feature
//!
//! The daemon announces a request's change on every session of its chain,
//! so one copy of each event arrives per chain member: every update here is
//! keyed by the request id and idempotent. A request held by the user is
//! drawn in its root view (the highest chain member open in the TUI other
//! than the origin); every view above the origin shows a waiting row.
//!
//! BUG-648: the events reach the TUI from each open chain member's
//! transcript pages, so copies from different members' journals can be
//! applied in either order. An answered or cancelled request id stays
//! closed: a Pending copy applied after it never brings the request back.

use codelet_rpc_types::{RequestHolder, RequestKind, RequestRoute, RequestStatus, SessionId};

use super::AgentViewStore;

/// The pending requests seen on any chain, oldest first.
#[derive(Debug, Default)]
pub struct ChainRequests {
    pending: Vec<RequestRoute>,
    /// BUG-648: the ids of requests answered or cancelled; they stay closed.
    closed: std::collections::HashSet<String>,
}

impl AgentViewStore {
    /// Fold one request event. Returns `true` when the origin's prompt
    /// must be read for the user: the event moved the request to the user
    /// and the origin has no prompt read for this request id yet.
    pub fn apply_request_change(&mut self, route: &RequestRoute, status: &RequestStatus) -> bool {
        let requests = &mut self.chain_requests;
        if requests.closed.contains(&route.request_id) {
            return false;
        }
        let pending = &mut requests.pending;
        let at = pending
            .iter()
            .position(|known| known.request_id == route.request_id);
        if !matches!(status, RequestStatus::Pending) {
            if let Some(at) = at {
                pending.remove(at);
            }
            requests.closed.insert(route.request_id.clone());
            self.clear_origin_prompt(route);
            return false;
        }
        match at {
            Some(at) if pending[at] == *route => return false,
            Some(at) => pending[at] = route.clone(),
            None => pending.push(route.clone()),
        }
        route.holder == RequestHolder::User && !self.origin_prompt_read(route)
    }

    /// The oldest request held by the user that `view` is the root view
    /// of, when its origin's prompt has been read.
    pub fn relayed_request_for(&self, view: &SessionId) -> Option<&RequestRoute> {
        self.chain_requests.pending.iter().find(|route| {
            route.holder == RequestHolder::User
                && self.root_view_of(route) == Some(view)
                && self.origin_prompt_read(route)
        })
    }

    /// The pending requests `view` waits on: those whose chain has `view`
    /// above the origin, except the one its view shows as the prompt.
    pub fn waiting_requests_for(&self, view: &SessionId) -> Vec<&RequestRoute> {
        let relayed = self.relayed_request_for(view).map(|r| &r.request_id);
        self.chain_requests
            .pending
            .iter()
            .filter(|route| route.chain.iter().skip(1).any(|member| member == view))
            .filter(|route| Some(&route.request_id) != relayed)
            .collect()
    }

    /// True when a request below `session` on its chain is pending.
    pub fn waits_on_request(&self, session: &SessionId) -> bool {
        self.chain_requests
            .pending
            .iter()
            .any(|route| route.chain.iter().skip(1).any(|member| member == session))
    }

    /// Who asks, from the origin up to the root view: e.g.
    /// "sub-agent #3 of sub-agent #2 of bot #1".
    pub fn request_asker(&self, route: &RequestRoute) -> String {
        let root = self.root_view_of(route).cloned();
        let upto = root
            .as_ref()
            .and_then(|root| route.chain.iter().position(|member| member == root))
            .unwrap_or(route.chain.len().saturating_sub(1));
        route
            .chain
            .iter()
            .take(upto + 1)
            .map(|member| self.chain_member_label(member, Some(member) == root.as_ref()))
            .collect::<Vec<_>>()
            .join(" of ")
    }

    /// The waiting row for `route`, e.g. "Waiting: sub-agent #2 asks for
    /// approval; agent #1 holds it".
    pub fn waiting_row(&self, route: &RequestRoute) -> String {
        let asks = match route.kind {
            RequestKind::Question => "asks a question",
            RequestKind::Approval => "asks for approval",
            RequestKind::HumanAction => "waits on a human action",
        };
        let holder = match &route.holder {
            RequestHolder::User => "the user".to_string(),
            RequestHolder::Session(holder) => {
                self.chain_member_label(holder, self.root_view_of(route) == Some(holder))
            }
        };
        let origin = self.chain_member_label(&route.origin, false);
        format!("Waiting: {origin} {asks}; {holder} holds it")
    }

    /// The highest chain member open in the TUI, other than the origin.
    fn root_view_of<'a>(&self, route: &'a RequestRoute) -> Option<&'a SessionId> {
        route
            .chain
            .iter()
            .skip(1)
            .rev()
            .find(|member| self.session_context_for(member).is_some())
    }

    /// "bot #N" / "agent #N" for the root, "sub-agent #N" otherwise; the
    /// first 8 characters of the id for a session not open in the TUI.
    fn chain_member_label(&self, session: &SessionId, is_root: bool) -> String {
        let slot = match self.session_index_for(session) {
            Some((slot, _)) => format!("#{slot}"),
            None => session.value.chars().take(8).collect(),
        };
        let role = match (is_root, self.is_bot_session(session)) {
            (true, true) => "bot",
            (true, false) => "agent",
            (false, _) => "sub-agent",
        };
        format!("{role} {slot}")
    }

    /// True when the origin's question or approval slot holds this request.
    fn origin_prompt_read(&self, route: &RequestRoute) -> bool {
        let id = Some(&route.request_id);
        self.hitl_prompt_for(&route.origin)
            .map(|slot| &slot.route.request_id)
            == id
            || self
                .pause_route_for(&route.origin)
                .map(|known| &known.request_id)
                == id
    }

    /// Drop the origin's prompt for an answered or cancelled request.
    fn clear_origin_prompt(&mut self, route: &RequestRoute) {
        let id = Some(&route.request_id);
        if self
            .hitl_prompt_for(&route.origin)
            .map(|slot| &slot.route.request_id)
            == id
        {
            self.clear_hitl_prompt(&route.origin);
        }
        if self
            .pause_route_for(&route.origin)
            .map(|known| &known.request_id)
            == id
        {
            self.clear_pause_state(&route.origin);
        }
    }
}
