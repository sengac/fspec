//! BUG-601: the blocklist rules each session turned off.
//!
//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! A rule turned off in the blocklist view is off only for the session the
//! view was opened from (product owner, 2026-11-08). codelet-sessions saves
//! the set with the session and hands it here when the user changes it and
//! when the session is resumed; the command and file checks drop the
//! session's turned-off rules before they match.

use super::config::BlocklistConfig;
use std::collections::{BTreeSet, HashMap};
use std::sync::{LazyLock, RwLock};

static SESSION_DISABLED_RULES: LazyLock<RwLock<HashMap<uuid::Uuid, BTreeSet<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Make `rules` the rule ids turned off for `session_id` (an empty set turns
/// every rule on again).
pub fn set_session_disabled_rules(session_id: uuid::Uuid, rules: BTreeSet<String>) {
    let mut guard = match SESSION_DISABLED_RULES.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if rules.is_empty() {
        guard.remove(&session_id);
    } else {
        guard.insert(session_id, rules);
    }
}

/// The rule ids turned off for `session_id`.
#[must_use]
pub fn session_disabled_rules(session_id: uuid::Uuid) -> BTreeSet<String> {
    let guard = match SESSION_DISABLED_RULES.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.get(&session_id).cloned().unwrap_or_default()
}

/// The session closed: forget its turned-off rules here (the session's saved
/// copy brings them back when it is resumed).
pub fn forget_session_disabled_rules(session_id: uuid::Uuid) {
    let mut guard = match SESSION_DISABLED_RULES.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.remove(&session_id);
}

/// Forget every session's turned-off rules (a process that starts again).
pub fn clear_session_disabled_rules() {
    let mut guard = match SESSION_DISABLED_RULES.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clear();
}

/// The blocklist `session_id` is checked against: the system and project
/// rules without the ones the session turned off.
#[must_use]
pub fn session_blocklist_config(session_id: uuid::Uuid) -> BlocklistConfig {
    let root = super::middleware::middleware_project_root();
    let mut config = super::middleware::load_blocklist_config(root.as_deref());
    let disabled = session_disabled_rules(session_id);
    config.rules.retain(|rule| !disabled.contains(&rule.id));
    config
}
