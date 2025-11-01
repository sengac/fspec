import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface ShowDeletedOptions {
  workUnitId: string;
  cwd?: string;
}

interface DeletedItem {
  id: number;
  text: string;
  deletedAt?: string;
}

interface ShowDeletedResult {
  success: boolean;
  deletedItems: DeletedItem[];
  totalDeleted: number;
}

export async function showDeleted(
  options: ShowDeletedOptions
): Promise<ShowDeletedResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Collect all deleted items into a flat array
  const deletedRules = (workUnit.rules || [])
    .filter(r => r.deleted)
    .map(r => ({ id: r.id, text: r.text, deletedAt: r.deletedAt }));

  const deletedExamples = (workUnit.examples || [])
    .filter(e => e.deleted)
    .map(e => ({ id: e.id, text: e.text, deletedAt: e.deletedAt }));

  const deletedQuestions = (workUnit.questions || [])
    .filter(q => q.deleted)
    .map(q => ({ id: q.id, text: q.text, deletedAt: q.deletedAt }));

  const deletedArchitectureNotes = (workUnit.architectureNotes || [])
    .filter(n => n.deleted)
    .map(n => ({ id: n.id, text: n.text, deletedAt: n.deletedAt }));

  const deletedItems = [
    ...deletedRules,
    ...deletedExamples,
    ...deletedQuestions,
    ...deletedArchitectureNotes,
  ];

  return {
    success: true,
    deletedItems,
    totalDeleted: deletedItems.length,
  };
}

export function registerShowDeletedCommand(program: Command): void {
  program
    .command('show-deleted')
    .description('Display all soft-deleted items in a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await showDeleted({ workUnitId });

        if (result.totalDeleted === 0) {
          console.log(chalk.dim('No deleted items found'));
          return;
        }

        console.log(
          chalk.bold(
            `\nDeleted items in ${workUnitId} (${result.totalDeleted} total):`
          )
        );

        // Display all deleted items
        result.deletedItems.forEach(item => {
          const timestamp = item.deletedAt
            ? chalk.dim(` (deleted: ${item.deletedAt})`)
            : '';
          console.log(chalk.red(`  [${item.id}] ${item.text}${timestamp}`));
        });

        console.log(''); // Empty line for spacing
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to show deleted items:'),
          error.message
        );
        process.exit(1);
      }
    });
}
