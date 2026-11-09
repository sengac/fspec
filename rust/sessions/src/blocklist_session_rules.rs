//! BUG-601: turning a blocklist rule off and on for one session.
//!
//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! A rule turned off is off only for that session, and it is saved with the
//! session (its manifest), so it stays off after a daemon restart and resume
//! (product owner, 2026-11-08). codelet-tools enforces the in-memory copy.

use crate::SessionManager;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;

impl SessionManager {
    /// The rule ids the live session `session_id` turned off (sorted).
    pub(crate) fn session_blocklist_disabled_rules(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, String> {
        let session = self.get_session(&session_id.value)?;
        Ok(codelet_tools::blocklist::session_disabled_rules(session.id)
            .into_iter()
            .collect())
    }

    /// Turn `rule_id` (a rule `blocklist_list` lists) off or on for the live
    /// session `session_id`: saved with the session first, then enforced.
    pub(crate) fn set_session_blocklist_rule_enabled(
        &self,
        session_id: &SessionId,
        rule_id: &str,
        enabled: bool,
    ) -> Result<Vec<String>, String> {
        let session = self.get_session(&session_id.value)?;
        if !self.blocklist_list()?.iter().any(|rule| rule.id == rule_id) {
            return Err(format!("the blocklist has no rule \"{rule_id}\""));
        }
        let manifest = codelet_core::persistence::update_session_manifest(session.id, |manifest| {
            if enabled {
                manifest.blocklist_disabled_rules.remove(rule_id);
            } else {
                manifest
                    .blocklist_disabled_rules
                    .insert(rule_id.to_string());
            }
            Ok(())
        })
        .map_err(|error| {
            format!("cannot save the session's turned-off blocklist rules: {error}")
        })?;
        codelet_tools::blocklist::set_session_disabled_rules(
            session.id,
            manifest.blocklist_disabled_rules.clone(),
        );
        Ok(manifest.blocklist_disabled_rules.into_iter().collect())
    }
}
