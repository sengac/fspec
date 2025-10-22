import { mkdir, writeFile, rm } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import React from 'react';
import { render } from 'ink';
import {
  getAgentById,
  type AgentConfig,
  AGENT_REGISTRY,
} from '../utils/agentRegistry';
import { generateAgentDoc } from '../utils/templateGenerator';
import { getSlashCommandTemplate } from '../utils/slashCommandTemplate';
import { detectAgents } from '../utils/agentDetection';
import { AgentSelector } from '../components/AgentSelector';

/**
 * Install fspec for multiple agents
 */
export async function installAgents(
  cwd: string,
  agentIds: string[]
): Promise<void> {
  // Validate agent IDs
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (!agent) {
      const { AGENT_REGISTRY } = await import('../utils/agentRegistry');
      const validAgents = AGENT_REGISTRY.filter(a => a.available)
        .map(a => `  - ${a.id}: ${a.description}`)
        .join('\n');
      throw new Error(
        `Unknown agent: ${agentId}.\n\nValid agent IDs:\n${validAgents}`
      );
    }
  }

  // Remove old agent files if switching agents (idempotent behavior)
  await removeOtherAgentFiles(cwd, agentIds);

  // Install each agent
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (agent) {
      await installAgentFiles(cwd, agent);
    }
  }
}

/**
 * Remove files for agents NOT in the installation list
 */
async function removeOtherAgentFiles(
  cwd: string,
  keepAgentIds: string[]
): Promise<void> {
  const { AGENT_REGISTRY } = await import('../utils/agentRegistry');

  for (const agent of AGENT_REGISTRY) {
    // Skip agents we're installing
    if (keepAgentIds.includes(agent.id)) {
      continue;
    }

    // Remove root stub file
    const rootStubPath = join(cwd, agent.rootStubFile);
    try {
      await rm(rootStubPath, { force: true });
    } catch {
      // File may not exist
    }

    // Remove full doc file
    const docPath = join(cwd, 'spec', agent.docTemplate);
    try {
      await rm(docPath, { force: true });
    } catch {
      // File may not exist
    }

    // Remove slash command file (NOT the entire directory)
    const filename =
      agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
    const slashCmdFile = join(cwd, agent.slashCommandPath, filename);
    try {
      await rm(slashCmdFile, { force: true });
    } catch {
      // File may not exist
    }
  }
}

/**
 * Install files for a single agent
 */
export async function installAgentFiles(
  cwd: string,
  agent: AgentConfig
): Promise<void> {
  // 1. Install full documentation (spec/AGENT.md)
  await installFullDoc(cwd, agent);

  // 2. Install slash command file
  await installSlashCommand(cwd, agent);
}

/**
 * Install full documentation file
 */
async function installFullDoc(cwd: string, agent: AgentConfig): Promise<void> {
  const specDir = join(cwd, 'spec');
  await mkdir(specDir, { recursive: true });

  const docPath = join(specDir, agent.docTemplate);

  // Generate agent-specific documentation
  const content = await generateAgentDoc(agent);

  await writeFile(docPath, content, 'utf-8');
}

/**
 * Install slash command file
 */
async function installSlashCommand(
  cwd: string,
  agent: AgentConfig
): Promise<void> {
  const commandsDir = join(cwd, agent.slashCommandPath);
  await mkdir(commandsDir, { recursive: true });

  // Use correct file extension based on format
  const filename =
    agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
  const commandPath = join(commandsDir, filename);

  // Generate slash command content
  const content = generateSlashCommandContent(agent);

  await writeFile(commandPath, content, 'utf-8');
}

/**
 * Generate slash command content
 */
function generateSlashCommandContent(agent: AgentConfig): string {
  if (agent.slashCommandFormat === 'toml') {
    // TOML format for Gemini CLI, Qwen Code
    return `[command]
name = "fspec - Load Project Context"
description = "Load fspec workflow and ACDD methodology"

# fspec Command - Load Full Context

Run these commands to load fspec context:

1. fspec --help
2. fspec help specs
3. fspec help work
4. fspec help discovery

Then read the comprehensive guide at spec/${agent.docTemplate} for full ACDD workflow.
`;
  }

  // Markdown format (most agents)
  // Use embedded template (no filesystem dependency)
  const template = getSlashCommandTemplate();

  // Return the full template (which should be 1000+ lines)
  return template;
}

export function registerInitCommand(program: Command): void {
  program
    .command('init')
    .description('Initialize fspec for AI coding agents')
    .option(
      '--agent <agent>',
      'Agent ID (can be repeated for multiple agents)',
      (value, previous: string[] = []) => {
        return [...previous, value];
      },
      []
    )
    .action(async (options: { agent: string[] }) => {
      try {
        const cwd = process.cwd();
        let agentIds: string[];

        // Interactive mode: no --agent flag provided
        if (options.agent.length === 0) {
          // Check if stdin supports raw mode (required for interactive selection)
          if (!process.stdin.isTTY || !process.stdin.setRawMode) {
            throw new Error(
              'Interactive mode requires a TTY. Use --agent flag instead:\n' +
                '  fspec init --agent=claude\n' +
                '  fspec init --agent=cursor --agent=claude'
            );
          }

          // Auto-detect agents in current directory
          const detected = await detectAgents(cwd);
          const availableAgents = AGENT_REGISTRY.filter(a => a.available);

          // Show interactive selector
          const selectedAgent = await new Promise<string>(resolve => {
            const { waitUntilExit } = render(
              React.createElement(AgentSelector, {
                agents: availableAgents,
                preSelected: detected,
                onSubmit: selected => {
                  resolve(selected);
                },
              })
            );
            void waitUntilExit();
          });

          agentIds = [selectedAgent];
        } else {
          // CLI mode: --agent flag(s) provided
          agentIds = options.agent;
        }

        // Install agents using new multi-agent system
        await installAgents(cwd, agentIds);

        // Success message (only for CLI mode, interactive mode shows in React component)
        if (options.agent.length > 0) {
          const agentNames = agentIds.join(', ');
          console.log(
            chalk.green(
              `✓ Installed fspec for ${agentNames}\n\nNext steps:\nRun /fspec in your AI agent to activate`
            )
          );
        }
        process.exit(0);
      } catch (error: any) {
        console.error(chalk.red('✗ Init failed:'), error.message);
        process.exit(1);
      }
    });
}
