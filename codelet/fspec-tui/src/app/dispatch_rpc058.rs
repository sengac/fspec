//! RPC-058 — App::dispatch routing for the `/schedule …` slash command
//! family.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-054 /
//! RPC-055 / RPC-056 / RPC-057 patterns: spawn a tokio task that
//! awaits the backend round-trip, route the response back through the
//! action bus via `Action::EmitSessionNotice` so the notice lands on
//! the right `SessionContext` even if the user switched tabs while
//! the RPC was in flight.
//!
//! Flow:
//!   1. `handle_slash_schedule_help` — entry point from the slash
//!      command palette (popup pick of `/schedule`). With no current
//!      session it's a silent no-op. Otherwise emits the static
//!      `[schedule] Usage: …` help block into the focused session's
//!      scrollback.
//!   2. `handle_schedule_subcommand` — fans out an
//!      `Action::ScheduleSubcommandParsed(sub)` to the matching
//!      handle_schedule_* helper.
//!   3. `handle_schedule_add` / `handle_schedule_list` /
//!      `handle_schedule_pause` / `handle_schedule_resume` /
//!      `handle_schedule_remove` — spawn the corresponding backend RPC
//!      and route the response into the focused session's scrollback
//!      via `Action::EmitSessionNotice`.

use codelet_rpc_types::ScheduledJob;
use tokio::task::JoinHandle;

use crate::components::Action;

use super::schedule_parser::ScheduleSubcommand;
use super::state::App;

/// Static usage block for `/schedule` (matches TS USAGE_TEXT).
const USAGE_TEXT: &str = "[schedule] Usage: /schedule <subcommand> [options]\n\
  add <name> --cron <expr> --tz <zone> [--role <r> --prompt <p>] [--command <cmd>] [--overlap skip|queue]\n\
  list\n\
  pause <name>\n\
  resume <name>\n\
  remove <name>";

impl App {
    /// RPC-058: `/schedule` slash-command popup pick entry point. With
    /// no current session this is a silent no-op (matches /merge-worktree
    /// and /clear). Otherwise emits the static help block.
    pub(crate) fn handle_slash_schedule_help(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        let _ = self.action_tx.send(Action::EmitSessionNotice(
            session_id,
            USAGE_TEXT.to_string(),
        ));
    }

    /// RPC-058: route a parsed [`ScheduleSubcommand`] to the matching
    /// handler. Help is a synchronous notice; the others spawn the
    /// backend round-trip via `handle_schedule_*`.
    pub(crate) fn handle_schedule_subcommand(&mut self, sub: ScheduleSubcommand) {
        match sub {
            ScheduleSubcommand::Help => self.handle_slash_schedule_help(),
            ScheduleSubcommand::Add {
                name,
                cron,
                timezone,
                job_type,
                role,
                prompt,
                command,
                overlap_policy,
            } => self.handle_schedule_add(ScheduledJob {
                name,
                cron,
                timezone,
                job_type,
                status: "active".to_string(),
                created_at: None,
                last_run_at: None,
                last_run_status: None,
                role,
                prompt,
                command,
                overlap_policy,
            }),
            ScheduleSubcommand::List => self.handle_schedule_list(),
            ScheduleSubcommand::Pause { name } => self.handle_schedule_pause(name),
            ScheduleSubcommand::Resume { name } => self.handle_schedule_resume(name),
            ScheduleSubcommand::Remove { name } => self.handle_schedule_remove(name),
        }
    }

    pub(crate) fn handle_schedule_add(&mut self, job: ScheduledJob) {
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
            let text = match backend.schedule_add(job).await {
                Ok(j) => format_schedule_added(&j),
                Err(e) => format_schedule_error("add", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_schedule_list(&mut self) {
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
            let text = match backend.schedule_list().await {
                Ok(rows) => format_schedule_list(&rows),
                Err(e) => format_schedule_error("list", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_schedule_pause(&mut self, name: String) {
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
            let text = match backend.schedule_pause(name.clone()).await {
                Ok(_job) => format_schedule_state("paused", &name),
                Err(e) => format_schedule_error("pause", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_schedule_resume(&mut self, name: String) {
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
            let text = match backend.schedule_resume(name.clone()).await {
                Ok(_job) => format_schedule_state("resumed", &name),
                Err(e) => format_schedule_error("resume", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_schedule_remove(&mut self, name: String) {
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
            let text = match backend.schedule_remove(name.clone()).await {
                Ok(()) => format_schedule_state("removed", &name),
                Err(e) => format_schedule_error("remove", &e.to_string()),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    /// Route RPC-058 Action variants through their helpers. Called from
    /// the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_rpc058(&mut self, action: &Action) -> bool {
        match action {
            Action::ScheduleSubcommandParsed(sub) => {
                self.handle_schedule_subcommand(sub.clone());
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

fn format_schedule_added(job: &ScheduledJob) -> String {
    format!(
        "[schedule] added \"{}\" ({}, {}, {})",
        job.name, job.job_type, job.cron, job.timezone
    )
}

fn format_schedule_state(state: &str, name: &str) -> String {
    format!("[schedule] {state} \"{name}\"")
}

fn format_schedule_list(rows: &[ScheduledJob]) -> String {
    if rows.is_empty() {
        return "[schedule] No schedules configured.".to_string();
    }
    let mut out = format!("[schedule] {} schedule(s)", rows.len());
    for row in rows {
        let last_run = row.last_run_at.as_deref().unwrap_or("-");
        out.push('\n');
        out.push_str(&format!(
            "  {}    {}    {}    {}    {}    {}",
            row.name, row.cron, row.timezone, row.job_type, row.status, last_run
        ));
    }
    out
}

fn format_schedule_error(sub: &str, e: &str) -> String {
    format!("[error] /schedule {sub}: {e}")
}
