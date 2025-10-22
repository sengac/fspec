import { rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import { getAgentById } from '../utils/agentRegistry';
import { detectAgents } from '../utils/agentDetection';

interface RemoveOptions {
  keepConfig: boolean;
}

/**
 * Remove fspec initialization files
 */
export async function removeInitFiles(
  cwd: string,
  options: RemoveOptions
): Promise<void> {
  // Detect which agent is installed
  const detectedAgentId = await detectInstalledAgent(cwd);

  if (!detectedAgentId) {
    throw new Error(
      'No fspec agent installation detected. Nothing to remove.'
    );
  }

  const agent = getAgentById(detectedAgentId);
  if (!agent) {
    throw new Error(`Unknown agent: ${detectedAgentId}`);
  }

  // Remove agent-specific files
  await removeAgentFiles(cwd, agent.id);

  // Optionally remove config file
  if (!options.keepConfig) {
    const configPath = join(cwd, 'spec', 'fspec-config.json');
    await rm(configPath, { force: true });
  }
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
async function removeAgentFiles(cwd: string, agentId: string): Promise<void> {
  const agent = getAgentById(agentId);
  if (!agent) {
    return;
  }

  // Remove spec/AGENT.md (e.g., spec/CLAUDE.md)
  const docPath = join(cwd, 'spec', agent.docTemplate);
  await rm(docPath, { force: true });

  // Remove slash command file (e.g., .claude/commands/fspec.md)
  const filename =
    agent.slashCommandFormat === 'toml' ? 'fspec.toml' : 'fspec.md';
  const slashCmdPath = join(cwd, agent.slashCommandPath, filename);
  await rm(slashCmdPath, { force: true });
}

export function registerRemoveInitFilesCommand(program: Command): void {
  program
    .command('remove-init-files')
    .description('Remove fspec initialization files')
    .action(async () => {
      try {
        const cwd = process.cwd();

        // TODO: Add interactive prompt for keepConfig
        // For now, hardcode to false (remove everything)
        const options: RemoveOptions = { keepConfig: false };

        await removeInitFiles(cwd, options);

        console.log(chalk.green('✓ Successfully removed fspec init files'));
        process.exit(0);
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to remove init files:'), error.message);
        process.exit(1);
      }
    });
}
