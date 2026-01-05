import { rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import React from 'react';
import { render } from 'ink';
import { getAgentById } from '../utils/agentRegistry';
import { detectAgents } from '../utils/agentDetection';
import { ConfirmPrompt } from '../components/ConfirmPrompt';

interface RemoveOptions {
  keepConfig: boolean;
}

interface ExecuteRemoveOptions {
  keepConfig?: boolean;
  promptKeepConfig?: (message: string) => Promise<boolean>;
}

interface RemoveResult {
  filesRemoved: string[];
}

/**
 * Remove fspec initialization files
 */
export async function removeInitFiles(
  cwd: string,
  options: RemoveOptions
): Promise<string[]> {
  // Detect which agent is installed
  const detectedAgentId = await detectInstalledAgent(cwd);

  if (!detectedAgentId) {
    throw new Error('No fspec agent installation detected. Nothing to remove.');
  }

  const agent = getAgentById(detectedAgentId);
  if (!agent) {
    throw new Error(`Unknown agent: ${detectedAgentId}`);
  }

  const filesRemoved: string[] = [];

  // Remove agent-specific files
  const agentFiles = await removeAgentFiles(cwd, agent.id);
  filesRemoved.push(...agentFiles);

  // Optionally remove config file
  if (!options.keepConfig) {
    const configPath = join(cwd, 'spec', 'fspec-config.json');
    await rm(configPath, { force: true });
    filesRemoved.push('spec/fspec-config.json');
  }

  return filesRemoved;
}

/**
 * Execute remove-init-files with optional interactive prompt
 * (Testable function for action handler)
 */
export async function executeRemoveInitFiles(
  options: ExecuteRemoveOptions = {}
): Promise<RemoveResult> {
  const cwd = process.cwd();
  let keepConfig: boolean;

  // If keepConfig is explicitly set, use it
  if (options.keepConfig !== undefined) {
    keepConfig = options.keepConfig;
  }
  // If promptKeepConfig function provided (for testing), use it
  else if (options.promptKeepConfig) {
    keepConfig = await options.promptKeepConfig('Keep spec/fspec-config.json?');
  }
  // Otherwise, show interactive prompt
  else {
    keepConfig = await showKeepConfigPrompt();
  }

  const filesRemoved = await removeInitFiles(cwd, { keepConfig });
  return { filesRemoved };
}

/**
 * Show interactive prompt for keeping config
 */
async function showKeepConfigPrompt(): Promise<boolean> {
  return new Promise<boolean>(resolve => {
    const { waitUntilExit } = render(
      React.createElement(ConfirmPrompt, {
        message: 'Keep spec/fspec-config.json?',
        confirmLabel: 'Yes',
        cancelLabel: 'No',
        onSubmit: (confirmed: boolean) => {
          resolve(confirmed);
        },
      }),
      {
        // Enable mouse events (trackpad, scroll wheel, clicks)
        stdin: process.stdin,
        stdout: process.stdout,
        // Enable incremental rendering to reduce flickering by only updating changed lines
        incrementalRendering: true,
        // Increase FPS from default 30 to 60 for smoother rendering
        maxFps: 60,
      }
    );
    void waitUntilExit();
  });
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
 * Remove files for a specific agent
 */
async function removeAgentFiles(
  cwd: string,
  agentId: string
): Promise<string[]> {
  const agent = getAgentById(agentId);
  if (!agent) {
    return [];
  }

  const filesRemoved: string[] = [];

  // Remove spec/AGENT.md (e.g., spec/CLAUDE.md)
  const docPath = join(cwd, 'spec', agent.docTemplate);
  await rm(docPath, { force: true });
  filesRemoved.push(`spec/${agent.docTemplate}`);

  // Remove slash command file (e.g., .claude/commands/fspec.md)
  const filename =
    agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
  const slashCmdPath = join(cwd, agent.slashCommandPath, filename);
  await rm(slashCmdPath, { force: true });
  filesRemoved.push(`${agent.slashCommandPath}${filename}`);

  return filesRemoved;
}

export function registerRemoveInitFilesCommand(program: Command): void {
  program
    .command('remove-init-files')
    .description('Remove fspec initialization files')
    .option(
      '--keep-config',
      'Keep spec/fspec-config.json (remove only agent files)'
    )
    .option(
      '--no-keep-config',
      'Remove all files including spec/fspec-config.json'
    )
    .action(async (options: { keepConfig?: boolean }) => {
      try {
        // Execute remove-init-files with explicit flags or interactive prompt
        const result = await executeRemoveInitFiles({
          keepConfig: options.keepConfig,
        });

        // Show success message with details
        console.log(chalk.green('✓ Successfully removed fspec init files'));
        if (result.filesRemoved.length > 0) {
          result.filesRemoved.forEach(file => {
            console.log(chalk.dim(`  - ${file}`));
          });
        }

        process.exit(0);
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to remove init files:'),
          error.message
        );
        process.exit(1);
      }
    });
}
