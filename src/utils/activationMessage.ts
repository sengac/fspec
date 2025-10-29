/**
 * Activation Message Generator
 *
 * Generates agent-specific activation instructions for fspec init success output.
 * Customizes message based on agent capabilities and slash command format.
 */

import type { AgentConfig } from './agentRegistry';

/**
 * Get agent-specific activation message for init success output
 */
export function getActivationMessage(agent: AgentConfig): string {
  // Handle known agents with specific patterns
  if (agent.id === 'claude') {
    return 'Run /fspec in Claude Code to activate';
  }

  if (agent.id === 'codex' || agent.id === 'codex-cli') {
    const agentName = agent.id === 'codex-cli' ? 'Codex CLI' : 'Codex';
    return `Run /prompts:fspec in ${agentName} to activate`;
  }

  if (agent.id === 'cursor') {
    return 'Open .cursor/commands/ in Cursor to activate';
  }

  if (agent.id === 'aider') {
    return 'Add .aider/ to your Aider configuration to activate';
  }

  // Handle IDE/extension agents (Cline, Windsurf, Copilot, etc.)
  if (agent.category === 'ide' || agent.category === 'extension') {
    return `Open ${agent.slashCommandPath} in ${agent.name} to activate`;
  }

  // Handle CLI agents (Gemini, Qwen, etc.)
  if (agent.category === 'cli' && agent.slashCommandPath) {
    return `Add ${agent.slashCommandPath} to your ${agent.name} configuration to activate`;
  }

  // Fallback for unknown/default agents
  return 'Refer to your AI agent documentation to activate fspec';
}
