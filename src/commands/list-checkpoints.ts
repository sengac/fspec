/**
 * List all checkpoints with visual indicators (emoji)
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { listCheckpoints as listCheckpointsUtil } from '../utils/git-checkpoint.js';

export interface ListCheckpointsOptions {
  workUnitId: string;
  cwd: string;
}

export interface CheckpointDisplay {
  name: string;
  timestamp: string;
  displayIcon: string;
  isAutomatic: boolean;
}

export async function listCheckpoints(
  options: ListCheckpointsOptions
): Promise<{
  checkpoints: CheckpointDisplay[];
}> {
  const { workUnitId, cwd } = options;

  try {
    const checkpoints = await listCheckpointsUtil(workUnitId, cwd);

    if (checkpoints.length === 0) {
      console.log(chalk.yellow(`No checkpoints found for ${workUnitId}`));
      return { checkpoints: [] };
    }

    console.log(chalk.cyan(`\nCheckpoints for ${workUnitId}:\n`));

    const displayCheckpoints = checkpoints.map(cp => {
      const icon = cp.isAutomatic ? '🤖' : '📌';
      const typeLabel = cp.isAutomatic
        ? chalk.gray('(automatic)')
        : chalk.blue('(manual)');

      console.log(`${icon}  ${chalk.bold(cp.name)} ${typeLabel}`);
      console.log(chalk.gray(`   Created: ${cp.timestamp}`));
      console.log('');

      return {
        name: cp.name,
        timestamp: cp.timestamp,
        displayIcon: icon,
        isAutomatic: cp.isAutomatic,
      };
    });

    return { checkpoints: displayCheckpoints };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(chalk.red(`✗ Failed to list checkpoints: ${errorMessage}`));
    throw error;
  }
}

async function listCheckpointsCommand(workUnitId: string): Promise<void> {
  try {
    await listCheckpoints({
      workUnitId,
      cwd: process.cwd(),
    });

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}

export function registerListCheckpointsCommand(program: Command): void {
  program
    .command('list-checkpoints')
    .description('List all checkpoints for a work unit')
    .argument('<work-unit-id>', 'Work unit ID (e.g., AUTH-001)')
    .action(listCheckpointsCommand);
}
