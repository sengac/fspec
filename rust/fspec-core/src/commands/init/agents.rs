//! Terminal-free agent queries for the CLI bridge (selector list, detection)
//! and the per-agent activation message.

use std::path::Path;

use super::registry::{
    Agent,
    Category::{Cli, Extension, Ide},
    AGENT_REGISTRY,
};

/// Public, terminal-free summary of an available agent, used by the CLI
/// bridge to build the interactive selector list without duplicating the
/// inlined registry. Mirrors the fields the TS `AgentSelector` reads from
/// `AgentConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// All agents offered in the interactive selector (port of
/// `getAvailableAgents()` — `AGENT_REGISTRY.filter(a => a.available)`),
/// preserving registry order.
pub fn available_agents() -> Vec<AgentInfo> {
    AGENT_REGISTRY
        .iter()
        .filter(|a| a.available)
        .map(|a| AgentInfo {
            id: a.id.to_string(),
            name: a.name.to_string(),
            description: a.description.to_string(),
        })
        .collect()
}

/// Detect agents already in use under `project_root` by probing each agent's
/// `detection_paths` (port of `detectAgents` — `src/utils/agentDetection.ts`).
/// Returns the matching agent ids in registry order; an agent is recorded at
/// most once even if multiple of its detection paths exist.
pub fn detect_agents(project_root: &Path) -> Vec<String> {
    AGENT_REGISTRY
        .iter()
        .filter(|a| {
            a.detection_paths
                .iter()
                .any(|rel| project_root.join(rel).exists())
        })
        .map(|a| a.id.to_string())
        .collect()
}

/// Agent-specific activation message (port of getActivationMessage,
/// `src/utils/activationMessage.ts`).
pub(super) fn activation_message(agent: &Agent) -> String {
    match agent.id {
        "claude" => "Run /fspec in Claude Code to activate".to_string(),
        "codex" => "Run /prompts:fspec in Codex to activate".to_string(),
        "codex-cli" => "Run /prompts:fspec in Codex CLI to activate".to_string(),
        "cursor" => "Open .cursor/commands/ in Cursor to activate".to_string(),
        "aider" => "Add .aider/ to your Aider configuration to activate".to_string(),
        _ => match agent.category {
            Ide | Extension => {
                format!(
                    "Open {} in {} to activate",
                    agent.slash_command_path, agent.name
                )
            }
            Cli if !agent.slash_command_path.is_empty() => format!(
                "Add {} to your {} configuration to activate",
                agent.slash_command_path, agent.name
            ),
            _ => "Refer to your AI agent documentation to activate fspec".to_string(),
        },
    }
}
