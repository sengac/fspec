//! WT-018: the one writer of `spec/work-units.json`.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Every command that changes the board writes it through
//! [`write_work_units`] (the locked-file writers refuse the board, so none
//! can bypass it). When the command runs in an isolated copy whose
//! `.fspec/state-root` pointer names a session, the writer compares the
//! board on disk (read under the spec lock the command holds) with the
//! board it writes and records the session's id on what this write
//! changes:
//!
//! - a card new to the board gets `createdInSession`;
//! - every object appended to one of a card's arrays (a `stateHistory`
//!   entry, a rule, an example, a question, an architecture note, an
//!   assumption, ...) gets `sessionId`;
//! - any other change to an existing card (its user story, estimate,
//!   title, dependencies or attachments, an answered or removed item)
//!   adds the id to the card's `editedInSessions` list.
//!
//! A write made without a pointer (the main checkout) records nothing.
//! Discarding a session's worktree reads these marks to list exactly the
//! cards the session created, moved and edited
//! ([`crate::session_cards`]).

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;

/// The card field naming the session that created the card.
pub const CREATED_IN_SESSION: &str = "createdInSession";
/// The field naming the session that appended an object to a card's array.
pub const SESSION_ID: &str = "sessionId";
/// The card field listing the sessions that edited the card.
pub const EDITED_IN_SESSIONS: &str = "editedInSessions";

/// Write `board` as the board of the project rooted at `project_root`,
/// recording the session id of the project's state-root pointer (if any)
/// on everything this write changes. The caller holds the spec lock.
pub fn write_work_units<T: Serialize>(
    project_root: &Path,
    board: &T,
) -> Result<(), FspecCoreError> {
    let roots = crate::io::spec_paths::roots(project_root)?;
    let path = roots.work_units();
    let Some(session) = roots.session_id() else {
        return crate::io::locked_file::write_board_json(&path, board);
    };
    let mut after = serde_json::to_value(board).map_err(|e| FspecCoreError::InvalidArgs {
        command: "write_work_units",
        reason: format!("failed to serialize the board: {e}"),
    })?;
    let before = read_board(&path)?;
    stamp_session(&mut after, before.as_ref(), session);
    crate::io::locked_file::write_board_json(&path, &after)
}

/// The board on disk, or `None` when there is none yet.
fn read_board(path: &Path) -> Result<Option<Value>, FspecCoreError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "write_work_units",
                source,
            })
        }
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| crate::io::json_error::parse_json_diagnostic("work-units.json", &raw, &e))
}

/// Record `session` on every card of `after` that differs from `before`.
fn stamp_session(after: &mut Value, before: Option<&Value>, session: &str) {
    let before_cards = before
        .and_then(|board| board.get("workUnits"))
        .and_then(Value::as_object);
    let Some(cards) = after.get_mut("workUnits").and_then(Value::as_object_mut) else {
        return;
    };
    for (id, card) in cards.iter_mut() {
        let old = before_cards.and_then(|cards| cards.get(id));
        if old == Some(&*card) {
            continue;
        }
        let Some(card) = card.as_object_mut() else {
            continue;
        };
        stamp_appended_objects(card, old.and_then(Value::as_object), session);
        match old {
            None => {
                card.insert(CREATED_IN_SESSION.to_string(), Value::from(session));
            }
            Some(_) => add_editing_session(card, session),
        }
    }
}

/// Give every object appended to one of `card`'s arrays since `old` the
/// field `sessionId`.
fn stamp_appended_objects(
    card: &mut Map<String, Value>,
    old: Option<&Map<String, Value>>,
    session: &str,
) {
    for (key, value) in card.iter_mut() {
        let Some(items) = value.as_array_mut() else {
            continue;
        };
        let kept = old
            .and_then(|old| old.get(key))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for item in items.iter_mut().skip(kept) {
            if let Some(item) = item.as_object_mut() {
                item.insert(SESSION_ID.to_string(), Value::from(session));
            }
        }
    }
}

/// Add `session` to the card's `editedInSessions` list once.
fn add_editing_session(card: &mut Map<String, Value>, session: &str) {
    let sessions = card
        .entry(EDITED_IN_SESSIONS.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(sessions) = sessions {
        if !sessions.iter().any(|known| known == session) {
            sessions.push(Value::from(session));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn an_unchanged_card_is_not_marked() {
        let before = json!({"workUnits": {"A-1": {"id": "A-1", "rules": [{"id": 0}]}}});
        let mut after = before.clone();
        stamp_session(&mut after, Some(&before), "s1");
        assert_eq!(after, before);
    }

    #[test]
    fn only_objects_appended_by_this_write_are_marked() {
        let before = json!({"workUnits": {"A-1": {"id": "A-1", "rules": [{"id": 0}], "attachments": ["a"]}}});
        let mut after = json!({"workUnits": {"A-1": {"id": "A-1", "rules": [{"id": 0}, {"id": 1}], "attachments": ["a", "b"]}}});
        stamp_session(&mut after, Some(&before), "s1");
        assert_eq!(
            after,
            json!({"workUnits": {"A-1": {
                "id": "A-1",
                "rules": [{"id": 0}, {"id": 1, "sessionId": "s1"}],
                "attachments": ["a", "b"],
                "editedInSessions": ["s1"]
            }}})
        );
    }

    #[test]
    fn a_session_is_listed_once_among_the_editors() {
        let before = json!({"workUnits": {"A-1": {"id": "A-1", "estimate": 1, "editedInSessions": ["s0", "s1"]}}});
        let mut after = json!({"workUnits": {"A-1": {"id": "A-1", "estimate": 2, "editedInSessions": ["s0", "s1"]}}});
        stamp_session(&mut after, Some(&before), "s1");
        assert_eq!(
            after["workUnits"]["A-1"]["editedInSessions"],
            json!(["s0", "s1"])
        );
    }

    /// WT-018: no fspec-core source hands the board's path to a locked-file
    /// writer (those refuse it at runtime, so such a command would fail);
    /// every board write goes through [`write_work_units`].
    #[test]
    fn no_source_writes_the_board_path_with_a_locked_file_writer() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let binding =
            regex::Regex::new(r"let (\w+) = crate::io::spec_paths::work_units\(").unwrap();
        let mut pending = vec![src];
        let mut offenders = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "rs") || path.ends_with("io/board.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for name in binding.captures_iter(&text).map(|c| c[1].to_string()) {
                    let write = regex::Regex::new(&format!(
                        r"(write_json_atomic|write_json_atomic_trailing_newline|write_bytes_atomic)\(&?{name}\b"
                    ))
                    .unwrap();
                    if write.is_match(&text) {
                        offenders.push(format!("{} ({name})", path.display()));
                    }
                }
                if text.contains("_atomic(&crate::io::spec_paths::work_units(") {
                    offenders.push(path.display().to_string());
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "board written past write_work_units: {offenders:?}"
        );
    }
}
