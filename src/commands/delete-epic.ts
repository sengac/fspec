import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import { fileManager } from '../utils/file-manager';

interface Epic {
  id: string;
  [key: string]: unknown;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface Prefix {
  prefix: string;
  epicId?: string;
  [key: string]: unknown;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

interface WorkUnit {
  id: string;
  epic?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function deleteEpic(options: {
  epicId: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();
  const epicsFile = join(cwd, 'spec', 'epics.json');
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // LOCK-002: Use fileManager.transaction() for atomic read-modify-write
    await fileManager.transaction(epicsFile, async epicsData => {
      if (!epicsData.epics[options.epicId]) {
        throw new Error(`Epic ${options.epicId} not found`);
      }

      // Delete the epic
      delete epicsData.epics[options.epicId];
    });

    // LOCK-002: Update prefixes.json - remove epic references
    try {
      await fileManager.transaction(prefixesFile, async prefixesData => {
        for (const prefix of Object.values(prefixesData.prefixes)) {
          if (prefix.epicId === options.epicId) {
            delete prefix.epicId;
          }
        }
      });
    } catch {
      // No prefixes file yet
    }

    // LOCK-002: Update work-units.json - remove epic references
    try {
      await fileManager.transaction(workUnitsFile, async workUnitsData => {
        for (const workUnit of Object.values(workUnitsData.workUnits)) {
          if (workUnit.epic === options.epicId) {
            delete workUnit.epic;
          }
        }
      });
    } catch {
      // No work units file yet
    }

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to delete epic: ${error.message}`);
    }
    throw error;
  }
}

export function registerDeleteEpicCommand(program: Command): void {
  program
    .command('delete-epic')
    .description('Delete an epic')
    .argument('<epicId>', 'Epic ID to delete')
    .option('--force', 'Force deletion even if work units are associated')
    .action(async (epicId: string, options: { force?: boolean }) => {
      try {
        await deleteEpic({
          epicId,
          force: options.force,
        });
        console.log(chalk.green(`✓ Epic ${epicId} deleted successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to delete epic:'), error.message);
        process.exit(1);
      }
    });
}
