/**
 * Agent Runtime Configuration
 *
 * Provides runtime agent detection and output formatting based on agent capabilities.
 * Priority: FSPEC_AGENT env var > spec/fspec-config.json > safe default
 */

import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { getAgentById, type AgentConfig } from './agentRegistry';

export interface AgentRuntimeConfig {
  agent: string;
}

/**
 * Get agent configuration from environment or config file
 * Priority: FSPEC_AGENT env var > spec/fspec-config.json > safe default
 */
export function getAgentConfig(cwd: string): AgentConfig {
  // Priority 1: FSPEC_AGENT environment variable
  const envAgent = process.env.FSPEC_AGENT;
  if (envAgent) {
    const agent = getAgentById(envAgent);
    if (agent) {
      return agent;
    }
  }

  // Priority 2: spec/fspec-config.json
  const configPath = join(cwd, 'spec', 'fspec-config.json');
  if (existsSync(configPath)) {
    try {
      const configContent = readFileSync(configPath, 'utf-8');
      const config: AgentRuntimeConfig = JSON.parse(configContent);
      const agent = getAgentById(config.agent);
      if (agent) {
        return agent;
      }
    } catch (err) {
      // Fall through to default
    }
  }

  // Priority 3: Safe default (plain text, no system-reminders)
  // Return a safe default agent config
  return {
    id: 'default',
    name: 'Default',
    description: 'Safe default for unknown agents',
    slashCommandPath: '',
    slashCommandFormat: 'markdown',
    supportsSystemReminders: false,
    supportsMetaCognition: false,
    docTemplate: '',
    detectionPaths: [],
    available: true,
    category: 'cli',
  };
}

/**
 * Write agent configuration to spec/fspec-config.json
 */
export function writeAgentConfig(cwd: string, agentId: string): void {
  const specDir = join(cwd, 'spec');
  const configPath = join(specDir, 'fspec-config.json');

  // Ensure spec directory exists
  if (!existsSync(specDir)) {
    mkdirSync(specDir, { recursive: true });
  }

  // Write config file
  const config: AgentRuntimeConfig = { agent: agentId };
  writeFileSync(configPath, JSON.stringify(config, null, 2), 'utf-8');
}

/**
 * Format output based on agent capabilities
 * - Claude: <system-reminder> tags
 * - IDE agents (Cursor/Cline): **⚠️ IMPORTANT:**
 * - CLI agents (Aider/Gemini): **IMPORTANT:**
 */
export function formatAgentOutput(agent: AgentConfig, message: string): string {
  // Claude Code gets system-reminder tags
  if (agent.supportsSystemReminders) {
    return `<system-reminder>\n${message}\n</system-reminder>`;
  }

  // IDE agents (Cursor, Cline, Windsurf, etc.) get bold text with emoji
  if (agent.category === 'ide' || agent.category === 'extension') {
    return `**⚠️ IMPORTANT:** ${message}`;
  }

  // CLI-only agents (Aider, Gemini, etc.) get plain bold text
  return `**IMPORTANT:** ${message}`;
}
