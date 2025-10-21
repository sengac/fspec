/**
 * Cleanup old checkpoints, keeping most recent N
 */

import chalk from 'chalk';
import { cleanupCheckpoints as cleanupCheckpointsUtil } from '../utils/git-checkpoint.js';

export interface CleanupCheckpointsOptions {
  workUnitId: string;
  keepLast: number;
  cwd: string;
}

export async function cleanupCheckpoints(
  options: CleanupCheckpointsOptions
): Promise<{
  deletedCount: number;
  preservedCount: number;
  summary: {
    deleted: Array<{ name: string; timestamp: string }>;
    preserved: Array<{ name: string; timestamp: string }>;
  };
}> {
  const { workUnitId, keepLast, cwd } = options;

  try {
    const result = await cleanupCheckpointsUtil(workUnitId, cwd, keepLast);

    console.log(
      chalk.cyan(
        `\nCleaning up checkpoints for ${workUnitId} (keeping last ${keepLast})...\n`
      )
    );

    if (result.deletedCount > 0) {
      console.log(chalk.red(`Deleted ${result.deletedCount} checkpoint(s):`));
      result.deleted.forEach(cp => {
        console.log(chalk.gray(`  - ${cp.name} (${cp.timestamp})`));
      });
      console.log('');
    }

    if (result.preservedCount > 0) {
      console.log(
        chalk.green(`Preserved ${result.preservedCount} checkpoint(s):`)
      );
      result.preserved.forEach(cp => {
        console.log(chalk.gray(`  - ${cp.name} (${cp.timestamp})`));
      });
      console.log('');
    }

    console.log(
      chalk.green(
        `✓ Cleanup complete: ${result.deletedCount} deleted, ${result.preservedCount} preserved`
      )
    );

    return {
      deletedCount: result.deletedCount,
      preservedCount: result.preservedCount,
      summary: {
        deleted: result.deleted.map(cp => ({
          name: cp.name,
          timestamp: cp.timestamp,
        })),
        preserved: result.preserved.map(cp => ({
          name: cp.name,
          timestamp: cp.timestamp,
        })),
      },
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(
      chalk.red(`✗ Failed to cleanup checkpoints: ${errorMessage}`)
    );
    throw error;
  }
}
