//! App::dispatch routing for the BlocklistView surface that
//! backs the `/blocklist` slash command. Introduced: RPC-056.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-054
//! patterns: spawn a tokio task that awaits the backend round-trip,
//! route the response back through the action bus, fold it into the
//! Navigator's `BlocklistView` on the App task.

use tokio::task::JoinHandle;

use codelet_rpc_types::BlocklistRuleInfo;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-056: open the blocklist view + kick off the initial list
    /// fetch. The Navigator's `apply_action` arm flips `active_view`
    /// to `Blocklist` BEFORE this helper runs so the first render
    /// after the dispatch shows the (possibly empty) view while the
    /// backend call resolves.
    pub(crate) fn handle_open_blocklist_view(&mut self) {
        // Reset selection so a previous open's selection doesn't leak
        // back in.
        self.navigator.blocklist = crate::views::BlocklistView::new();
        self.spawn_blocklist_list();
    }

    /// RPC-056: close the blocklist view. The Navigator's
    /// `apply_action` arm flips back to `Agent`.
    pub(crate) fn handle_close_blocklist_view(&mut self) {
        // Nothing to clean up — the disabled-rule set lives on
        // AgentViewStore and persists across close/reopen by design.
    }

    /// RPC-056: spawn `backend.blocklist_list()` and route the result
    /// into the view via `Action::BlocklistRulesLoaded`. Mirrors
    /// `spawn_list_provider_credentials` from dispatch_provider_settings.rs.
    fn spawn_blocklist_list(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.blocklist_list().await {
                Ok(rules) => {
                    let _ = action_tx.send(Action::BlocklistRulesLoaded(rules));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "blocklist_list failed");
                    let _ = action_tx.send(Action::BlocklistRulesLoaded(Vec::new()));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-056: fold a `blocklist_list` response into the view.
    pub(crate) fn handle_blocklist_rules_loaded(&mut self, rules: Vec<BlocklistRuleInfo>) {
        self.navigator.blocklist.set_rules(rules);
    }

    /// RPC-056: fold a per-session toggle into the AgentViewStore.
    /// The focused session id is resolved at dispatch time so the
    /// store entry lands on the right SessionContext even if the
    /// user has switched sessions while the BlocklistView is open
    /// (the view itself is global, not per-session — but the
    /// disabled set is per-session, mirroring TS Ink's AgentView
    /// component state lift).
    pub(crate) fn handle_toggle_blocklist_rule(&mut self, rule_id: String) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        self.agent_view_store
            .toggle_blocklist_rule(&session, rule_id);
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
            _ => return false,
        }
        true
    }
}
