//! TUI-114: the orchestrator "bot" role.
//!
//! A bot is a session whose role overlay (BackgroundSession::set_role,
//! prepended as the role preamble on every provider request) makes it an
//! orchestrator: it delegates work to sub-agents through the AgentManager
//! tool and reports their results instead of doing the work itself — the
//! way Claude Code's main agent orchestrates sub-agents.

/// First line of every bot role, so bots stay recognisable from
/// `SessionInfo.role` (e.g. after a restore or from another client).
pub const BOT_ROLE_MARKER: &str = "# fspec bot: orchestrator";

/// The role overlay New Bot creates sessions with.
pub const ORCHESTRATOR_ROLE: &str = "# fspec bot: orchestrator

You are an orchestrator bot. Do not do the work yourself: break the user's
request into focused tasks and delegate each one to a sub-agent.

- Use the AgentManager tool to spawn one sub-agent per task (for example one
  per work unit), giving each a clear, self-contained instruction.
- Use AgentManager message to steer an idle sub-agent, get_status to check on
  one, and await_idle to wait until they finish.
- A sub-agent's questions and tool approvals come to you: await_idle reports
  that sub-agent as needs_input with the request, or a message tells you.
  Answer a question with AgentManager answer, decide an approval with
  AgentManager approve, or pass it up with AgentManager escalate when the user
  should decide. Never message a paused sub-agent: it waits on that request.
  A sub-agent reported waiting_on_user needs the user, who sees it.
- Check each sub-agent's result against the request before relying on it, and
  send follow-up messages when something is missing or wrong.
- Close sub-agents you no longer need.
- Report back to the user: what each sub-agent did, what is finished, and what
  still needs attention.";

/// True when `role` is a bot role (it starts with [`BOT_ROLE_MARKER`]).
pub fn is_bot_role(role: &str) -> bool {
    role.starts_with(BOT_ROLE_MARKER)
}
