import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface PrioritizeWorkUnitOptions {
  workUnitId: string;
  position?: 'top' | 'bottom' | number;
  before?: string;
  after?: string;
  cwd?: string;
}

interface PrioritizeWorkUnitResult {
  success: boolean;
}

export async function prioritizeWorkUnit(
  options: PrioritizeWorkUnitOptions
): Promise<PrioritizeWorkUnitResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];

  // Validate work unit is NOT in done status
  if (workUnit.status === 'done') {
    throw new Error(
      `Cannot prioritize work units in done column. Done items are ordered by completion time and cannot be manually reordered. Only backlog, specifying, testing, implementing, validating, blocked can be prioritized.`
    );
  }

  // Validate target work units exist if before/after specified
  if (options.before && !workUnitsData.workUnits[options.before]) {
    throw new Error(`Work unit '${options.before}' does not exist`);
  }
  if (options.after && !workUnitsData.workUnits[options.after]) {
    throw new Error(`Work unit '${options.after}' does not exist`);
  }

  // Validate cross-column prioritization (CRITICAL: cannot prioritize across different columns)
  if (options.before) {
    const beforeWorkUnit = workUnitsData.workUnits[options.before];
    if (beforeWorkUnit.status !== workUnit.status) {
      throw new Error(
        `Cannot prioritize across columns. ${options.workUnitId} (${workUnit.status}) and ${options.before} (${beforeWorkUnit.status}) are in different columns.`
      );
    }
  }
  if (options.after) {
    const afterWorkUnit = workUnitsData.workUnits[options.after];
    if (afterWorkUnit.status !== workUnit.status) {
      throw new Error(
        `Cannot prioritize across columns. ${options.workUnitId} (${workUnit.status}) and ${options.after} (${afterWorkUnit.status}) are in different columns.`
      );
    }
  }

  // Get the current column (state array) based on work unit status
  const currentStatus = workUnit.status;

  // Validate data integrity: work unit should be in the array matching its status
  if (!workUnitsData.states[currentStatus].includes(options.workUnitId)) {
    throw new Error(
      `Data integrity error: Work unit ${options.workUnitId} has status '${currentStatus}' but is not in states.${currentStatus} array. Run 'fspec repair-work-units' to fix data corruption.`
    );
  }

  // Remove work unit from current position (use filter to safely handle any duplicates)
  const column = workUnitsData.states[currentStatus].filter(
    id => id !== options.workUnitId
  );

  // Determine new position
  let newIndex = 0;

  if (options.position === 'top') {
    newIndex = 0;
  } else if (options.position === 'bottom') {
    newIndex = column.length;
  } else if (typeof options.position === 'number') {
    // Convert from 1-based (user input) to 0-based (array index)
    newIndex = options.position - 1;
    // Validate bounds
    if (newIndex < 0) {
      throw new Error(
        `Invalid position: ${options.position}. Position must be >= 1 (1-based index)`
      );
    }
    // Allow positions beyond array length (splice will insert at end)
  } else if (options.before) {
    newIndex = column.indexOf(options.before);
    if (newIndex === -1) {
      throw new Error(
        `Data integrity error: Work unit ${options.before} has status '${currentStatus}' but is not in states.${currentStatus} array. Run 'fspec repair-work-units' to fix data corruption.`
      );
    }
  } else if (options.after) {
    newIndex = column.indexOf(options.after);
    if (newIndex === -1) {
      throw new Error(
        `Data integrity error: Work unit ${options.after} has status '${currentStatus}' but is not in states.${currentStatus} array. Run 'fspec repair-work-units' to fix data corruption.`
      );
    }
    newIndex = newIndex + 1;
  }

  // Insert at new position
  column.splice(newIndex, 0, options.workUnitId);

  // Update states
  workUnitsData.states[currentStatus] = column;

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  return { success: true };
}

export function registerPrioritizeWorkUnitCommand(program: Command): void {
  program
    .command('prioritize-work-unit')
    .description('Reorder work units in any Kanban column except done')
    .argument('<workUnitId>', 'Work unit ID to prioritize')
    .option('--position <position>', 'Position: top, bottom, or numeric index')
    .option('--before <workUnitId>', 'Place before this work unit')
    .option('--after <workUnitId>', 'Place after this work unit')
    .action(
      async (
        workUnitId: string,
        options: { position?: string; before?: string; after?: string }
      ) => {
        try {
          const parsedPosition =
            options.position === 'top'
              ? 'top'
              : options.position === 'bottom'
                ? 'bottom'
                : options.position
                  ? parseInt(options.position, 10)
                  : undefined;
          await prioritizeWorkUnit({
            workUnitId,
            position: parsedPosition as 'top' | 'bottom' | number | undefined,
            before: options.before,
            after: options.after,
          });
          console.log(
            chalk.green(`✓ Work unit ${workUnitId} prioritized successfully`)
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to prioritize work unit:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
