//! App::dispatch routing for the BlocklistView surface that
//! backs the `/blocklist` slash command. Introduced: RPC-056.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-054
//! patterns: spawn a tokio task that awaits the backend round-trip,
//! route the response back through the action bus, fold it into the
//! Navigator's `BlocklistView` on the App task.
//!
//! BUG-601: a rule's on/off state lives in the daemon, for the session
//! the view was opened from. The view reads it when it opens, Toggle Rule
//! asks the daemon to change it, and only the daemon's answer is shown; a
//! failed call is a session error, never an empty or all-on view.

use tokio::task::JoinHandle;

use codelet_rpc_types::{BlocklistRuleInfo, SessionId};

use crate::components::Action;

use super::state::App;

/// BUG-601: Toggle Rule with no session to turn the rule off or on for.
const TOGGLE_WITHOUT_SESSION: &str =
    "Toggle Rule failed: open /blocklist from a session to turn its rules off or on";

impl App {
    /// RPC-056: open the blocklist view + kick off the initial list
    /// fetch. The Navigator's `apply_action` arm flips `active_view`
    /// to `Blocklist` BEFORE this helper runs so the first render
    /// after the dispatch shows the (possibly empty) view while the
    /// backend call resolves. BUG-601: the view remembers the session it
    /// was opened from and reads that session's turned-off rules.
    pub(crate) fn handle_open_blocklist_view(&mut self) {
        // Reset selection so a previous open's selection doesn't leak
        // back in.
        let session = self.agent_view_store.current_session().cloned();
        self.navigator.blocklist = crate::views::BlocklistView::new();
        self.navigator.blocklist.session = session.clone();
        self.spawn_blocklist_list();
        if let Some(session) = session {
            self.spawn_blocklist_disabled_rules(session);
        }
    }

    /// RPC-056: close the blocklist view. The Navigator's
    /// `apply_action` arm flips back to `Agent`.
    pub(crate) fn handle_close_blocklist_view(&mut self) {
        // Nothing to clean up: the turned-off rules live in the daemon.
    }

    /// RPC-056: spawn `backend.blocklist_list()` and route the result
    /// into the view via `Action::BlocklistRulesLoaded`. BUG-601: a
    /// failure is shown, not turned into an empty list.
    fn spawn_blocklist_list(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let action = match backend.blocklist_list().await {
                Ok(rules) => Action::BlocklistRulesLoaded(rules),
                Err(error) => Action::SessionOperationFailed(format!(
                    "Blocklist rules could not be loaded: {error}"
                )),
            };
            let _ = action_tx.send(action);
        });
        self.pending_tasks.push(handle);
    }

    /// BUG-601: read the rules the daemon has turned off for `session`.
    fn spawn_blocklist_disabled_rules(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let action = match backend.blocklist_disabled_rules(session.clone()).await {
                Ok(rules) => Action::BlocklistDisabledRulesLoaded(session, rules),
                Err(error) => Action::SessionOperationFailed(format!(
                    "Turned-off blocklist rules could not be loaded: {error}"
                )),
            };
            let _ = action_tx.send(action);
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-056: fold a `blocklist_list` response into the view.
    pub(crate) fn handle_blocklist_rules_loaded(&mut self, rules: Vec<BlocklistRuleInfo>) {
        self.navigator.blocklist.set_rules(rules);
    }

    /// BUG-601: Toggle Rule asks the daemon to turn `rule_id` off (when the
    /// daemon has it on) or on for the session the view was opened from;
    /// the daemon's answer replaces what the view shows.
    pub(crate) fn handle_toggle_blocklist_rule(&mut self, rule_id: String) {
        let Some(session) = self.navigator.blocklist.session.clone() else {
            self.dispatch(Action::SessionOperationFailed(
                TOGGLE_WITHOUT_SESSION.to_string(),
            ));
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let enabled = self
            .agent_view_store
            .blocklist_disabled_for(&session)
            .is_some_and(|rules| rules.contains(&rule_id));
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let action = match backend
                .blocklist_set_rule_enabled(session.clone(), rule_id, enabled)
                .await
            {
                Ok(rules) => Action::BlocklistDisabledRulesLoaded(session, rules),
                Err(error) => {
                    Action::SessionOperationFailed(format!("Toggle Rule failed: {error}"))
                }
            };
            let _ = action_tx.send(action);
        });
        self.pending_tasks.push(handle);
    }

    /// Route the RPC-056 Action variants through their helpers.
    /// Called from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_blocklist(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenBlocklistView => {
                self.handle_open_blocklist_view();
            }
            Action::CloseBlocklistView => {
                self.handle_close_blocklist_view();
            }
            Action::BlocklistRulesLoaded(rules) => {
                self.handle_blocklist_rules_loaded(rules.clone());
            }
            Action::ToggleBlocklistRule(id) => {
                self.handle_toggle_blocklist_rule(id.clone());
            }
            Action::BlocklistDisabledRulesLoaded(session, rules) => {
                self.agent_view_store
                    .set_blocklist_disabled(session.clone(), rules.clone());
            }
            _ => return false,
        }
        true
    }
}
