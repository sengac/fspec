//! TUI-124 — what closing a session will do, gathered before the close
//! dialog opens, and the exact words the dialog says about it.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature
//!
//! Words follow the kind of session (product owner, 2026-11-07): a bot
//! says "bot", every other session says "agent". The backend close is the
//! same for both kinds: it stops the running turn, keeps the saved history
//! (resumable with /resume) and leaves an isolated session's worktree until
//! /worktrees prune removes it. Closing a bot also closes its sub-agents
//! (product owner rule, 2026-11-07); closing an agent never does.

use codelet_rpc_types::SessionId;

/// The kind of session a dialog names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// Any session that is not a bot.
    Agent,
    /// A TUI-114 orchestrator bot (its role starts with the bot marker).
    Bot,
}

impl SessionKind {
    /// The lower-case word: "agent" / "bot".
    pub fn word(self) -> &'static str {
        match self {
            SessionKind::Agent => "agent",
            SessionKind::Bot => "bot",
        }
    }

    /// The capitalised word: "Agent" / "Bot".
    pub fn title_word(self) -> &'static str {
        match self {
            SessionKind::Agent => "Agent",
            SessionKind::Bot => "Bot",
        }
    }

    /// The kind of a session with `role`: a bot role makes it a bot.
    pub fn of_role(role: Option<&str>) -> Self {
        if role.is_some_and(codelet_sessions::bot::is_bot_role) {
            SessionKind::Bot
        } else {
            SessionKind::Agent
        }
    }
}

/// What this TUI knows of a sub-agent's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentStatus {
    /// Running or compacting.
    Running,
    /// Open here and not working.
    Idle,
    /// Not open in this TUI, so its status is not known.
    Unknown,
}

/// A sub-agent that closes together with its bot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingSubAgent {
    pub id: SessionId,
    pub status: SubAgentStatus,
    /// True when this TUI knows it works in a worktree.
    pub isolated: bool,
}

/// What the backend says about a session's relations, fetched when a
/// close is requested (the TUI's cached supervisors can lag a spawn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseRelations {
    /// The sessions it supervises (its sub-agents), in the backend's order.
    pub sub_agents: Vec<SessionId>,
    /// The sessions that supervise it (empty when it is not a sub-agent).
    pub started_by: Vec<SessionId>,
}

/// Everything the close dialog states, gathered before it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseFacts {
    /// The session being closed.
    pub session: SessionId,
    pub kind: SessionKind,
    /// The session itself is running or compacting.
    pub busy: bool,
    /// The session works in its own worktree.
    pub isolated: bool,
    /// The bot's sub-agents, closed first and in this order (always empty
    /// for an agent).
    pub closing_sub_agents: Vec<ClosingSubAgent>,
    /// How many sub-agents an agent started; they keep running (always 0
    /// for a bot, whose sub-agents close with it).
    pub sub_agents_kept: usize,
    /// The kinds of the sessions that started this one (empty when it is
    /// not a sub-agent); `None` when the starter is not open here.
    pub started_by: Vec<Option<SessionKind>>,
}

impl CloseFacts {
    /// An idle, non-isolated session with no sub-agents and no starter.
    pub fn new(session: SessionId, kind: SessionKind) -> Self {
        Self {
            session,
            kind,
            busy: false,
            isolated: false,
            closing_sub_agents: Vec::new(),
            sub_agents_kept: 0,
            started_by: Vec::new(),
        }
    }

    /// "Close Agent?" / "Close Bot?".
    pub fn title(&self) -> String {
        format!("Close {}?", self.kind.title_word())
    }

    fn sub_agent_running(&self) -> bool {
        self.closing_sub_agents
            .iter()
            .any(|sub| sub.status == SubAgentStatus::Running)
    }

    /// True when confirming stops work: the session or a sub-agent that
    /// closes with it is known to be running.
    pub fn stops_work(&self) -> bool {
        self.busy || self.sub_agent_running()
    }

    /// The confirm option: "Close", "Stop and Close", "Close All" or
    /// "Stop and Close All".
    pub fn close_label(&self) -> &'static str {
        match (self.stops_work(), self.closing_sub_agents.is_empty()) {
            (false, true) => "Close",
            (true, true) => "Stop and Close",
            (false, false) => "Close All",
            (true, false) => "Stop and Close All",
        }
    }

    /// The dialog's description lines, in order.
    pub fn lines(&self) -> Vec<String> {
        let word = self.kind.word();
        let mut lines = vec![if self.busy {
            format!("This {word} is working. Closing stops it and returns you to the board.")
        } else {
            format!("Closing ends this {word} and returns you to the board.")
        }];
        if let Some(line) = self.started_by_line() {
            lines.push(line);
        }
        if let Some(line) = self.sub_agents_line() {
            lines.push(line);
        }
        let closes_several = !self.closing_sub_agents.is_empty();
        lines.push(if closes_several {
            "All their history is kept: resume any of them later with /resume.".to_string()
        } else {
            "Its history is kept: resume it later with /resume.".to_string()
        });
        let any_sub_isolated = self.closing_sub_agents.iter().any(|sub| sub.isolated);
        if closes_several && (self.isolated || any_sub_isolated) {
            lines
                .push("Their worktrees are kept until you prune them with /worktrees.".to_string());
        } else if self.isolated {
            lines.push("Its worktree is kept until you prune it with /worktrees.".to_string());
        }
        lines
    }

    fn started_by_line(&self) -> Option<String> {
        let word = self.kind.word();
        match self.started_by.as_slice() {
            [] => None,
            [starter] => {
                let who = match starter {
                    Some(SessionKind::Bot) => "bot",
                    Some(SessionKind::Agent) => "agent",
                    None => "agent or bot",
                };
                Some(format!(
                    "Only this {word} is closed; the {who} that started it keeps running."
                ))
            }
            starters => Some(format!(
                "Only this {word} is closed; the {} that started it keep running.",
                starters.len()
            )),
        }
    }

    fn sub_agents_line(&self) -> Option<String> {
        match self.kind {
            SessionKind::Bot => {
                let count = self.closing_sub_agents.len();
                if count == 0 {
                    return Some("It has no sub-agents to close.".to_string());
                }
                let noun = if count == 1 {
                    "sub-agent"
                } else {
                    "sub-agents"
                };
                Some(format!(
                    "Its {count} {noun} will be closed too ({}).",
                    self.status_summary()
                ))
            }
            SessionKind::Agent => match self.sub_agents_kept {
                0 => None,
                1 => Some("Its 1 sub-agent keeps running.".to_string()),
                count => Some(format!("Its {count} sub-agents keep running.")),
            },
        }
    }

    fn status_summary(&self) -> String {
        let count = |status| {
            self.closing_sub_agents
                .iter()
                .filter(|sub| sub.status == status)
                .count()
        };
        let running = count(SubAgentStatus::Running);
        let unknown = count(SubAgentStatus::Unknown);
        match (running, unknown) {
            (0, 0) => "none running".to_string(),
            (running, 0) => format!("{running} running"),
            (running, unknown) => format!("{running} running, {unknown} status unknown"),
        }
    }
}
