//! WT-018: the cards an isolated session changed on the shared board, and
//! the result of moving them back when its worktree is discarded.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Every board write made from a session's worktree records the session's
//! id (stateHistory entries carry `sessionId`, a created card carries
//! `createdInSession`, any other edit adds the id to the card's
//! `editedInSessions`), so the daemon finds these cards exactly.

use serde::{Deserialize, Serialize};

/// One card the session created, moved or edited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCardChange {
    /// The card's id, e.g. `STORY-001`.
    pub work_unit_id: String,
    /// The card's title.
    pub title: String,
    /// The card's status on the board now.
    pub status: String,
    /// What the session did to it.
    pub change: SessionCardChangeKind,
}

/// What a session did to a card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCardChangeKind {
    /// The session created the card; it is kept on discard.
    Created,
    /// The session changed the card's status; `before` is its status
    /// before the session's first change (backlog when it had none).
    Moved { before: String },
    /// The session changed the card's status, and someone else changed it
    /// after the session's last change; it is never moved back.
    ChangedSince,
    /// The session edited the card without changing its status.
    Edited,
}

/// The result of moving one card back to its status before the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardMoveBack {
    /// The card's id.
    pub work_unit_id: String,
    /// What happened.
    pub outcome: CardMoveBackOutcome,
}

/// Whether a card was moved back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardMoveBackOutcome {
    /// The card is back in `status`.
    MovedBack { status: String },
    /// The move was refused; `error` is the status gate's message.
    Failed { error: String },
}
