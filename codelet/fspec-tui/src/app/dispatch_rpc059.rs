//! RPC-059 — App::dispatch routing for the `/loop …` slash command
//! family.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-058
//! pattern: spawn a tokio task that awaits the backend round-trip,
//! route the response back through the action bus via
//! `Action::EmitSessionNotice` so the notice lands on the right
//! `SessionContext` even if the user switched tabs while the RPC was
//! in flight.

use codelet_rpc_types::RegisteredLoop;
use tokio::task::JoinHandle;

use crate::components::Action;

use super::loop_parser::LoopSubcommand;
use super::state::App;

/// Static usage block for `/loop` (matches TS `handleLoopCommand`).
const USAGE_TEXT: &str = "[loop] Usage: /loop [interval] <prompt> | /loop cancel <id> | /loop list\n\
  Intervals: <N>s|<N>m|<N>h|<N>d (leading) OR ... every N <unit> (trailing)\n\
  Examples:\n\
    /loop 30s check the build\n\
    /loop 5m check deployment status\n\
    /loop check status every 2 hours\n\
    /loop check the build         # defaults to 10 minutes\n\
    /loop list\n\
    /loop cancel a1b2c3d4";

impl App {
    /// RPC-059: `/loop` slash-command popup pick entry point. With no
    /// current session this is a silent no-op (matches /schedule).
    /// Otherwise emits the static help block.
    pub(crate) fn handle_slash_loop_help(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        let _ = self
            .action_tx
            .send(Action::EmitSessionNotice(session_id, format_loop_help()));
    }

    /// RPC-059: route a parsed [`LoopSubcommand`] to the matching
    /// handler. Help is a synchronous notice; the others spawn the
    /// backend round-trip via `handle_loop_*`.
    pub(crate) fn handle_loop_subcommand(&mut self, sub: LoopSubcommand) {
        match sub {
            LoopSubcommand::Help => self.handle_slash_loop_help(),
            LoopSubcommand::Add {
                interval_seconds,
                prompt,
            } => self.handle_loop_add(interval_seconds, prompt),
            LoopSubcommand::List => self.handle_loop_list(),
            LoopSubcommand::Cancel { id } => self.handle_loop_cancel(id),
        }
    }

    pub(crate) fn handle_loop_add(&mut self, interval_seconds: u32, prompt: String) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let text = match backend
                .loop_add(sid_for_task.clone(), interval_seconds, prompt)
                .await
            {
                Ok(entry) => format_loop_added(&entry),
                Err(e) => format_loop_error("add", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_loop_list(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let text = match backend.loop_list(sid_for_task.clone()).await {
                Ok(rows) => format_loop_list(&rows),
                Err(e) => format_loop_error("list", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_loop_cancel(&mut self, id: String) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let text = match backend.loop_cancel(id.clone()).await {
                Ok(true) => format_loop_cancelled(&id),
                Ok(false) => format_loop_cancel_missing(&id),
                Err(e) => format_loop_error("cancel", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    /// Route RPC-059 Action variants through their helpers. Called from
    /// the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_rpc059(&mut self, action: &Action) -> bool {
        match action {
            Action::LoopSubcommandParsed(sub) => {
                self.handle_loop_subcommand(sub.clone());
            }
            _ => return false,
        }
        true
    }
}

// ─────────────────────────────────────────────────────────────────────
// Notice formatters — single-source the wire format so dispatch + tests
// can both reach them by name.
// ─────────────────────────────────────────────────────────────────────

/// Format `interval_seconds` as a human-readable string matching the
/// TS `formatInterval` helper.
fn format_interval(interval_seconds: u32) -> String {
    let n = interval_seconds;
    if n < 60 {
        return if n == 1 {
            "1 second".to_string()
        } else {
            format!("{n} seconds")
        };
    }
    if n < 3600 && n.is_multiple_of(60) {
        let m = n / 60;
        return if m == 1 {
            "1 minute".to_string()
        } else {
            format!("{m} minutes")
        };
    }
    if n < 3600 {
        return format!("{n} seconds");
    }
    if n < 86_400 && n.is_multiple_of(3600) {
        let h = n / 3600;
        return if h == 1 {
            "1 hour".to_string()
        } else {
            format!("{h} hours")
        };
    }
    if n.is_multiple_of(86_400) {
        let d = n / 86_400;
        return if d == 1 {
            "1 day".to_string()
        } else {
            format!("{d} days")
        };
    }
    if n.is_multiple_of(60) {
        let m = n / 60;
        return format!("{m} minutes");
    }
    format!("{n} seconds")
}

fn format_loop_added(entry: &RegisteredLoop) -> String {
    format!(
        "[loop] scheduled every {} [job: {}]",
        format_interval(entry.interval_seconds),
        entry.id
    )
}

fn format_loop_cancelled(id: &str) -> String {
    format!("[loop] cancelled {id}")
}

fn format_loop_cancel_missing(id: &str) -> String {
    format!("[error] /loop cancel: Loop \"{id}\" not found")
}

fn format_loop_list(entries: &[RegisteredLoop]) -> String {
    if entries.is_empty() {
        return "[loop] No active loops.".to_string();
    }
    let mut out = String::from("[loop] Active loops:\n");
    out.push_str(&format!(
        "{:<10}{:<30}{:<15}\n",
        "ID", "Prompt", "Interval"
    ));
    out.push_str(&"-".repeat(55));
    for entry in entries {
        let prompt = if entry.prompt.len() > 28 {
            format!("{}...", &entry.prompt[..25])
        } else {
            entry.prompt.clone()
        };
        out.push('\n');
        out.push_str(&format!(
            "{:<10}{:<30}{:<15}",
            entry.id,
            prompt,
            format_interval(entry.interval_seconds)
        ));
    }
    out
}

fn format_loop_error(sub: &str, e: &str) -> String {
    format!("[error] /loop {sub}: {e}")
}

/// Static multi-line USAGE_TEXT matching TS `handleLoopCommand` — kept
/// as a named helper so dispatch sites and tests can both reach the
/// wire format by name (mirrors the other `format_loop_*` formatters).
fn format_loop_help() -> String {
    USAGE_TEXT.to_string()
}
