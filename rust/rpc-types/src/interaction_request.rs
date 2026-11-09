//! AMGR-020: every question and approval a session raises is a request with
//! an id, its origin session, its kind and its parent chain (origin, its
//! parent, ... the root). Its holder is the one asked to answer: the
//! origin's parent, or the user for a root session or a human action.

use super::*;

/// What a request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum RequestKind {
    /// A `request_user_input` question.
    Question,
    /// A tool approval (blocklist, RLCD gate or security prompt).
    Approval,
    /// Something only a person can do (WebSearch "interact with the page,
    /// then press Enter", a /goal "human review needed" escalation).
    HumanAction,
}

/// Who holds a request, or who answered one: the user or a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum RequestHolder {
    User,
    Session(SessionId),
}

/// The identity and routing of one pending request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RequestRoute {
    /// Every reply names this id; it changes with every new request.
    pub request_id: String,
    /// The session that raised the request and waits on it.
    pub origin: SessionId,
    pub kind: RequestKind,
    /// The origin, its parent, ... the root, read when it was raised.
    pub chain: Vec<SessionId>,
    pub holder: RequestHolder,
}

/// A pending request with what it asks (`PauseState` or `HitlRequest`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRequest<T> {
    pub route: RequestRoute,
    pub payload: T,
}

/// What happened to a request, announced on every session of its chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    /// Raised and waiting on its holder.
    Pending,
    /// Answered by the user or a session.
    Answered { by: RequestHolder },
    /// Resolved as cancelled (the origin was interrupted or closed, or a
    /// newer request replaced it).
    Cancelled,
}
