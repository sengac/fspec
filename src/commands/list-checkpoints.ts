/**
 * List all checkpoints with visual indicators (emoji)
 */

import chalk from 'chalk';
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
