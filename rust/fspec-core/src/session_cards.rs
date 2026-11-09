//! WT-018: the cards an isolated session changed on the shared board, and
//! moving them back to their status before the session when its worktree
//! is discarded.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Every board write made from a session's worktree records the session's
//! id ([`crate::io::board`]): stateHistory entries carry `sessionId`, a
//! created card carries `createdInSession`, any other edit adds the id to
//! the card's `editedInSessions`. A card is listed as
//!
//! - created (and kept) when it carries the session in `createdInSession`;
//! - moved when one of its stateHistory entries carries the session: its
//!   status before the session is the state of the entry just before the
//!   session's first one, backlog when there is none;
//! - changed by someone else since when an entry without the session
//!   follows the session's last one: it is never moved back;
//! - edited when it lists the session in `editedInSessions` otherwise.
//!
//! Moving a card back is a discard transition of `update-work-unit-status`
//! run on the main checkout's board: it restores the exact status from
//! before the session, backlog included (the only path back to backlog),
//! still passes every other status gate, and records the move in the
//! card's stateHistory as a discard of the session.

use std::path::Path;

use codelet_rpc_types::{
    CardMoveBack, CardMoveBackOutcome, SessionCardChange, SessionCardChangeKind,
};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::board::{CREATED_IN_SESSION, EDITED_IN_SESSIONS, SESSION_ID};

const COMMAND: &str = "session-card-changes";

/// The cards session `session_id` created, moved or edited on the board of
/// the project rooted at `project_root`, in board order.
pub fn session_card_changes(
    project_root: &Path,
    session_id: &str,
) -> Result<Vec<SessionCardChange>, FspecCoreError> {
    let board = read_board(project_root)?;
    let mut changes = Vec::new();
    for (id, card) in cards(&board)? {
        let Some(change) = classify(id, card, session_id)? else {
            continue;
        };
        changes.push(SessionCardChange {
            work_unit_id: id.clone(),
            title: text_field(id, card, "title")?,
            status: text_field(id, card, "status")?,
            change: change.kind,
        });
    }
    Ok(changes)
}

/// Move every card session `session_id` moved (and nobody changed since)
/// back to its status before the session. A card whose move is refused is
/// reported with the gate's error; the other cards still move.
pub fn move_session_cards_back(
    project_root: &Path,
    session_id: &str,
) -> Result<Vec<CardMoveBack>, FspecCoreError> {
    let board = read_board(project_root)?;
    let mut moves = Vec::new();
    for (id, card) in cards(&board)? {
        let Some(Change {
            kind: SessionCardChangeKind::Moved { before },
            blocked_reason,
        }) = classify(id, card, session_id)?
        else {
            continue;
        };
        let outcome = if text_field(id, card, "status")? == before {
            CardMoveBackOutcome::MovedBack { status: before }
        } else {
            match crate::commands::update_work_unit_status::discard_transition(
                project_root,
                id,
                &before,
                blocked_reason.as_deref(),
                session_id,
            ) {
                Ok(_) => CardMoveBackOutcome::MovedBack { status: before },
                Err(error) => CardMoveBackOutcome::Failed {
                    error: error.to_string(),
                },
            }
        };
        moves.push(CardMoveBack {
            work_unit_id: id.clone(),
            outcome,
        });
    }
    Ok(moves)
}

/// What the session did to one card, with the blocked reason to restore
/// when the card was blocked before the session.
struct Change {
    kind: SessionCardChangeKind,
    blocked_reason: Option<String>,
}

fn classify(id: &str, card: &Value, session: &str) -> Result<Option<Change>, FspecCoreError> {
    let simple = |kind| {
        Ok(Some(Change {
            kind,
            blocked_reason: None,
        }))
    };
    if card.get(CREATED_IN_SESSION).and_then(Value::as_str) == Some(session) {
        return simple(SessionCardChangeKind::Created);
    }
    let history = match card.get("stateHistory") {
        None => &[][..],
        Some(Value::Array(history)) => history.as_slice(),
        Some(_) => return Err(malformed(id, "stateHistory is not an array")),
    };
    let by_session = |entry: &Value| entry.get(SESSION_ID).and_then(Value::as_str) == Some(session);
    if let (Some(first), Some(last)) = (
        history.iter().position(by_session),
        history.iter().rposition(by_session),
    ) {
        if last + 1 < history.len() {
            return simple(SessionCardChangeKind::ChangedSince);
        }
        let Some(earlier) = first.checked_sub(1).map(|index| &history[index]) else {
            return simple(SessionCardChangeKind::Moved {
                before: "backlog".to_string(),
            });
        };
        let before = earlier
            .get("state")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                malformed(
                    id,
                    &format!("stateHistory entry {} has no state", first - 1),
                )
            })?
            .to_string();
        let blocked_reason = (before == "blocked")
            .then(|| {
                earlier
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .flatten();
        return Ok(Some(Change {
            kind: SessionCardChangeKind::Moved { before },
            blocked_reason,
        }));
    }
    let edited = match card.get(EDITED_IN_SESSIONS) {
        None => false,
        Some(Value::Array(sessions)) => sessions.iter().any(|known| known == session),
        Some(_) => return Err(malformed(id, "editedInSessions is not an array")),
    };
    if edited {
        return simple(SessionCardChangeKind::Edited);
    }
    Ok(None)
}

fn read_board(project_root: &Path) -> Result<Value, FspecCoreError> {
    let path = crate::io::spec_paths::work_units(project_root)?;
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: COMMAND,
        source,
    })?;
    serde_json::from_str(&raw)
        .map_err(|e| crate::io::json_error::parse_json_diagnostic("work-units.json", &raw, &e))
}

fn cards(board: &Value) -> Result<&serde_json::Map<String, Value>, FspecCoreError> {
    board
        .get("workUnits")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            FspecCoreError::Message("work-units.json has no workUnits object".to_string())
        })
}

fn text_field(id: &str, card: &Value, field: &str) -> Result<String, FspecCoreError> {
    card.get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| malformed(id, &format!("{field} is missing or not a string")))
}

fn malformed(id: &str, what: &str) -> FspecCoreError {
    FspecCoreError::Message(format!("Work unit {id} is malformed: {what}"))
}
