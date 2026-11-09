//! BUG-601: a scripted daemon for the blocklist's per-session turned-off
//! rules. It keeps each session's turned-off rule ids, answers them sorted,
//! records every read and toggle, and fails when scripted to.

use super::MockBackend;
use anyhow::Result;
use codelet_rpc_types::SessionId;
use parking_lot::Mutex;
use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct BlocklistMock {
    disabled: Mutex<HashMap<String, BTreeSet<String>>>,
    disabled_requests: Mutex<Vec<String>>,
    toggle_requests: Mutex<Vec<(String, String, bool)>>,
    disabled_error: Mutex<Option<String>>,
    toggle_error: Mutex<Option<String>>,
}

impl MockBackend {
    /// The daemon has turned `rules` off for `session`.
    pub fn seed_blocklist_disabled(&self, session: &SessionId, rules: &[&str]) {
        self.blocklist.disabled.lock().insert(
            session.value.clone(),
            rules.iter().map(|rule| (*rule).to_string()).collect(),
        );
    }

    /// Every `blocklist_disabled_rules` call's session id, oldest first.
    pub fn blocklist_disabled_requests(&self) -> Vec<String> {
        self.blocklist.disabled_requests.lock().clone()
    }

    /// Every `blocklist_set_rule_enabled` call as (session, rule, enabled).
    pub fn blocklist_toggle_requests(&self) -> Vec<(String, String, bool)> {
        self.blocklist.toggle_requests.lock().clone()
    }

    /// Make every later `blocklist_disabled_rules` fail with `message`.
    pub fn fail_blocklist_disabled_reads(&self, message: &str) {
        *self.blocklist.disabled_error.lock() = Some(message.to_string());
    }

    /// Make every later `blocklist_set_rule_enabled` fail with `message`.
    pub fn fail_blocklist_toggles(&self, message: &str) {
        *self.blocklist.toggle_error.lock() = Some(message.to_string());
    }

    pub async fn blocklist_disabled_rules(&self, session_id: SessionId) -> Result<Vec<String>> {
        self.blocklist
            .disabled_requests
            .lock()
            .push(session_id.value.clone());
        if let Some(message) = self.blocklist.disabled_error.lock().clone() {
            return Err(anyhow::anyhow!(message));
        }
        Ok(self.turned_off(&session_id))
    }

    pub async fn blocklist_set_rule_enabled(
        &self,
        session_id: SessionId,
        rule_id: String,
        enabled: bool,
    ) -> Result<Vec<String>> {
        self.blocklist.toggle_requests.lock().push((
            session_id.value.clone(),
            rule_id.clone(),
            enabled,
        ));
        if let Some(message) = self.blocklist.toggle_error.lock().clone() {
            return Err(anyhow::anyhow!(message));
        }
        {
            let mut disabled = self.blocklist.disabled.lock();
            let rules = disabled.entry(session_id.value.clone()).or_default();
            if enabled {
                rules.remove(&rule_id);
            } else {
                rules.insert(rule_id);
            }
        }
        Ok(self.turned_off(&session_id))
    }

    fn turned_off(&self, session: &SessionId) -> Vec<String> {
        self.blocklist
            .disabled
            .lock()
            .get(&session.value)
            .map(|rules| rules.iter().cloned().collect())
            .unwrap_or_default()
    }
}
