//! UPD-002 — `/update` dispatch handler.
//!
//! Feature: spec/features/in-place-self-update-tui-command.feature
//!
//! Applies a parsed [`UpdateSubcommand`] by calling the shared
//! `codelet_fspec_core::update` engine (the SAME engine the `fspec update`
//! CLI subcommand uses — rule [0]). The update runs as a spawned tokio task
//! (the `/compact` fire-and-forget pattern) so the UI thread keeps
//! rendering while the download is in flight.
//!
//! The TUI MUST NOT auto-restart or exec itself after an update (rule [6])
//! — on success it reports the new version and instructs the user to
//! restart fspec to activate it.

use codelet_fspec_core::update::{UpdateConfig, UpdateOutcome};

use crate::components::Action;

use super::state::App;
use super::update_parser::{format_update_message, UpdateSubcommand};

impl App {
    /// Apply a `/update …` subcommand for the focused session.
    pub(crate) fn handle_update_subcommand(&mut self, sub: UpdateSubcommand) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            // No session: silently drop (matches the /compact arm).
            return;
        };

        match sub {
            UpdateSubcommand::Invalid(arg) => {
                self.navigator.agent.push_line(
                    &mut self.agent_view_store,
                    format!("[error] unknown /update argument: {arg}"),
                );
            }
            UpdateSubcommand::CheckOnly => {
                self.spawn_update_task(session_id, false);
            }
            UpdateSubcommand::CheckAndUpdate => {
                self.spawn_update_task(session_id, true);
            }
        }
    }

    /// Spawn the async update on the tokio runtime and report the result
    /// into the originating session's scrollback.
    fn spawn_update_task(&mut self, session_id: codelet_rpc_types::SessionId, perform: bool) {
        if tokio::runtime::Handle::try_current().is_err() {
            // No runtime (synchronous unit tests): emit a notice so the
            // call is observable and return.
            self.navigator.agent.push_line(
                &mut self.agent_view_store,
                "[error] /update unavailable (no tokio runtime)".to_string(),
            );
            return;
        }

        // The checking line lands immediately so the user sees feedback
        // while the release lookup is in flight.
        self.navigator.agent.push_line(
            &mut self.agent_view_store,
            "[update] checking for latest release…".to_string(),
        );

        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let cfg = UpdateConfig::for_production(env!("CARGO_PKG_VERSION"));
            let result = if perform {
                cfg.perform_update().await
            } else {
                match cfg.check_latest().await {
                    Ok(info) if !info.is_newer => Ok(UpdateOutcome::UpToDate {
                        version: info.version,
                    }),
                    Ok(info) => Ok(UpdateOutcome::Failed {
                        message: format!(
                            "newer release v{} available — run /update to install",
                            info.version
                        ),
                    }),
                    Err(e) => Err(e),
                }
            };
            let message = match result {
                Ok(outcome) => format_update_message(&cfg.current_version, &outcome),
                Err(e) => format!("error: {e}"),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(session_id, message));
        });
        self.pending_tasks.push(handle);
    }
}
