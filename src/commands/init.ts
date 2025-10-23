import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import { existsSync } from 'fs';
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
import { ConfirmPrompt } from '../components/ConfirmPrompt';
import { getActivationMessage } from '../utils/activationMessage';
import { writeAgentConfig } from '../utils/agentRuntimeConfig';

interface InstallOptions {
  shouldSwitch?: boolean;
  interactiveMode?: boolean;
  selectedAgent?: string;
}

interface ExecuteInitOptions {
  agentIds: string[];
  promptAgentSwitch?: (existingAgent: string, newAgent: string) => Promise<boolean>;
  trackConfigWrites?: (agent: string) => void;
}

interface InitResult {
  filesInstalled: string[];
  cancelled: boolean;
  success: boolean;
}

/**
 * Execute init with agent detection and switch prompting
 * (Testable function for action handler)
 */
export async function executeInit(
  options: ExecuteInitOptions
): Promise<InitResult> {
  const cwd = process.cwd();
  const { agentIds, promptAgentSwitch, trackConfigWrites } = options;

  // Detect existing agent
  const existingAgent = await detectInstalledAgent(cwd);

  // If existing agent differs from requested, prompt for switch
  if (existingAgent && existingAgent !== agentIds[0]) {
    const shouldSwitch = promptAgentSwitch
      ? await promptAgentSwitch(existingAgent, agentIds[0])
      : await showAgentSwitchPrompt(existingAgent, agentIds[0]);

    if (!shouldSwitch) {
      return {
        filesInstalled: [],
        cancelled: true,
        success: false,
      };
    }
  }

  // Install agents
  const filesInstalled = await installAgents(cwd, agentIds, {});

  // Track config writes (for testing)
  if (trackConfigWrites) {
    trackConfigWrites(agentIds[0]);
  }

  // Write agent config (removed duplicate from action handler)
  writeAgentConfig(cwd, agentIds[0]);

  return {
    filesInstalled,
    cancelled: false,
    success: true,
  };
}

/**
 * Install fspec for multiple agents
 */
export async function installAgents(
  cwd: string,
  agentIds: string[],
  options?: InstallOptions
): Promise<string[]> {
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

  // Check if user cancelled agent switch
  if (options?.shouldSwitch === false) {
    throw new Error('Agent switch cancelled by user');
  }

  // Remove old agent files if switching agents (idempotent behavior)
  await removeOtherAgentFiles(cwd, agentIds);

  const filesInstalled: string[] = [];

  // Install each agent
  for (const agentId of agentIds) {
    const agent = getAgentById(agentId);
    if (agent) {
      const files = await installAgentFiles(cwd, agent);
      filesInstalled.push(...files);
    }
  }

  return filesInstalled;
}

/**
 * Detect installed agent from config or file detection
 */
async function detectInstalledAgent(cwd: string): Promise<string | null> {
  // Try reading spec/fspec-config.json first
  const configPath = join(cwd, 'spec', 'fspec-config.json');

  if (existsSync(configPath)) {
    try {
      const content = await readFile(configPath, 'utf-8');
      const config = JSON.parse(content);
      if (config.agent) {
        return config.agent;
      }
    } catch {
      // Fall through to detection
    }
  }

  // Fall back to file detection
  const detected = await detectAgents(cwd);
  return detected.length > 0 ? detected[0] : null;
}

/**
 * Show interactive prompt for agent switching
 */
async function showAgentSwitchPrompt(
  existingAgent: string,
  newAgent: string
): Promise<boolean> {
  const existingAgentConfig = getAgentById(existingAgent);
  const newAgentConfig = getAgentById(newAgent);

  const existingName = existingAgentConfig?.name || existingAgent;
  const newName = newAgentConfig?.name || newAgent;

  return new Promise<boolean>(resolve => {
    const { waitUntilExit } = render(
      React.createElement(ConfirmPrompt, {
        message: `Switch from ${existingName} to ${newName}?`,
        confirmLabel: `Switch to ${newName}`,
        cancelLabel: 'Cancel',
        onSubmit: (confirmed: boolean) => {
          resolve(confirmed);
        },
      })
    );
    void waitUntilExit();
  });
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
): Promise<string[]> {
  const filesInstalled: string[] = [];

  // 1. Install full documentation (spec/AGENT.md)
  await installFullDoc(cwd, agent);
  filesInstalled.push(`spec/${agent.docTemplate}`);

  // 2. Install slash command file
  await installSlashCommand(cwd, agent);
  const filename =
    agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
  filesInstalled.push(`${agent.slashCommandPath}${filename}`);

  return filesInstalled;
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

        // Execute init with agent detection and switch prompting
        // In interactive mode, pass promptAgentSwitch to auto-confirm (skip second prompt)
        const result = await executeInit({
          agentIds,
          promptAgentSwitch: options.agent.length === 0 ? async () => true : undefined,
        });

        // Check if user cancelled
        if (result.cancelled) {
          console.log(chalk.yellow('Init cancelled'));
          process.exit(0);
        }

        // Success message (show for both CLI and interactive modes)
        if (result.success) {
          const agentNames = agentIds.join(', ');
          console.log(chalk.green(`✓ Installed fspec for ${agentNames}`));

          // Show detailed list of installed files
          if (result.filesInstalled.length > 0) {
            result.filesInstalled.forEach(file => {
              console.log(chalk.dim(`  - ${file}`));
            });
          }

          const agent = getAgentById(agentIds[0]);
          const activationMessage = agent
            ? getActivationMessage(agent)
            : 'Run /fspec in your AI agent to activate';
          console.log(chalk.green(`\nNext steps:\n${activationMessage}`));
        }
        process.exit(0);
      } catch (error: any) {
        console.error(chalk.red('✗ Init failed:'), error.message);
        process.exit(1);
      }
    });
}
