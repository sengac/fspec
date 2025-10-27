/**
 * Sync Version Command
 *
 * Compares embedded version in fspec.md with current package.json version.
 * If mismatch detected: updates both slash command and spec doc files, shows restart message, exits 1.
 * If match: emits tool configuration system-reminders (CONFIG-003), allows workflow to continue, exits 0.
 */

import type { Command } from 'commander';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import { getAgentById } from '../utils/agentRegistry';
import { installAgentFiles } from './init';
import { detectAgents } from '../utils/agentDetection';
import { getVersion } from '../utils/version';
import { checkTestCommand, checkQualityCommands } from './configure-tools';

interface SyncVersionOptions {
  embeddedVersion: string;
  cwd?: string;
}

/**
 * Get restart message based on agent capabilities
 */
function getRestartMessage(
  agentId: string,
  oldVersion: string,
  newVersion: string,
  supportsSystemReminders: boolean
): string {
  // Agent-specific restart instructions
  let restartInstructions = '';
  switch (agentId) {
    case 'claude':
      restartInstructions =
        'Exit this conversation and start a new one. Run /fspec again.';
      break;
    case 'cursor':
      restartInstructions = 'Restart Cursor and run /fspec again';
      break;
    case 'aider':
      restartInstructions = 'Restart Aider and run /fspec again';
      break;
    case 'windsurf':
      restartInstructions = 'Restart Windsurf and run /fspec again';
      break;
    case 'generic':
    default:
      restartInstructions = 'Restart your AI agent and run /fspec again';
  }

  const updateMessage = `⚠️  fspec files updated from v${oldVersion} to v${newVersion}

STOP: Please restart your AI agent to load the latest context.

${restartInstructions}

The old v${oldVersion} documentation may already be in your context.
Restarting ensures you get the complete v${newVersion} workflow.`;

  // Wrap in <system-reminder> tags if agent supports them
  if (supportsSystemReminders) {
    return `<system-reminder>\n${updateMessage}\n</system-reminder>`;
  }

  return updateMessage;
}

interface AgentDetectionResult {
  agentId: string;
  fromConfig: boolean;
}

/**
 * Detect agent from config or filesystem
 * Returns agent ID and whether it was detected from config (true) or filesystem (false)
 */
async function detectAgent(cwd: string): Promise<AgentDetectionResult | null> {
  // Try reading spec/fspec-config.json first
  const configPath = join(cwd, 'spec', 'fspec-config.json');

  if (existsSync(configPath)) {
    try {
      const content = await readFile(configPath, 'utf-8');
      const config = JSON.parse(content);
      if (config.agent) {
        return { agentId: config.agent, fromConfig: true };
      }
    } catch {
      // Fall through to filesystem detection
    }
  }

  // Fall back to filesystem detection (synchronous)
  const detected = detectAgents(cwd);
  if (detected.length > 0) {
    return { agentId: detected[0].agent.id, fromConfig: false };
  }

  return null;
}

/**
 * Sync version command implementation
 */
export async function syncVersion(
  options: SyncVersionOptions
): Promise<number> {
  const { embeddedVersion, cwd = process.cwd() } = options;

  try {
    // Read current package.json version
    const currentVersion = getVersion();

    // Compare versions
    if (embeddedVersion === currentVersion) {
      // Versions match - ensure agent config exists before checking tools
      // This ensures check functions get correct agent capabilities
      const detection = await detectAgent(cwd);
      if (detection && !detection.fromConfig) {
        // Agent detected from filesystem but not in config - write config
        const { writeAgentConfig } = await import(
          '../utils/agentRuntimeConfig.js'
        );
        writeAgentConfig(cwd, detection.agentId);
      }

      // Emit tool configuration checks
      // This helps onboard new AI agents by guiding them to configure tools
      // immediately after version update (per CONFIG-003)
      const testResult = await checkTestCommand(cwd);
      console.log(testResult.message);

      const qualityResult = await checkQualityCommands(cwd);
      console.log(qualityResult.message);

      // CONFIG-003: Fail if tool configuration is missing
      // This prevents AI from continuing workflow without proper setup
      if (testResult.message.includes('NO TEST COMMAND CONFIGURED')) {
        return 1;
      }

      return 0;
    }

    // Version mismatch detected - update files
    const detection = await detectAgent(cwd);

    if (!detection) {
      // No agent detected - warn and exit 1 to stop workflow
      console.warn(
        chalk.yellow(
          '⚠️  Cannot detect agent. Version mismatch detected.\nPlease run: fspec init'
        )
      );
      return 1;
    }

    const agent = getAgentById(detection.agentId);
    if (!agent) {
      console.warn(chalk.yellow('⚠️  Unknown agent detected.'));
      return 1;
    }

    // Update both slash command and spec doc files
    await installAgentFiles(cwd, agent);

    // Print restart message
    // Use agent-specific message if detected from config, generic if from filesystem
    const useGeneric = !detection.fromConfig;
    const restartMessage = getRestartMessage(
      useGeneric ? 'generic' : detection.agentId,
      embeddedVersion,
      currentVersion,
      agent.supportsSystemReminders
    );
    console.log(restartMessage);

    // Exit with code 1 to stop workflow
    return 1;
  } catch (error: any) {
    // If version check fails (permissions, missing files), warn but continue
    console.warn(chalk.yellow(`⚠️  Version check failed: ${error.message}`));
    return 0;
  }
}

/**
 * Register sync-version option
 */
export function registerSyncVersionCommand(program: Command): void {
  program.option(
    '--sync-version <version>',
    'Internal: Sync embedded version with package.json version'
  );
}
