import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface RepairWorkUnitsOptions {
  cwd?: string;
}

interface RepairWorkUnitsResult {
  success: boolean;
  repairs: string[];
  repaired: number;
}

export async function repairWorkUnits(
  options: RepairWorkUnitsOptions = {}
): Promise<RepairWorkUnitsResult> {
  const cwd = options.cwd || process.cwd();
  const repairs: string[] = [];

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Rebuild states index from scratch
  const newStates: Record<string, string[]> = {
    backlog: [],
    specifying: [],
    testing: [],
    implementing: [],
    validating: [],
    done: [],
    blocked: [],
  };

  // Place each work unit in correct state array
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    const status = workUnit.status;

    if (newStates[status]) {
      newStates[status].push(id);

      // Check if it was in wrong state before
      for (const [stateName, ids] of Object.entries(workUnitsData.states)) {
        if (stateName !== status && ids.includes(id)) {
          repairs.push(`Moved ${id} from ${stateName} to ${status}`);
        }
      }
    }
  }

  // Update states
  workUnitsData.states = newStates;

  // Repair bidirectional dependency relationships
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    // Repair blocks/blockedBy relationships
    if (workUnit.blocks) {
      for (const targetId of workUnit.blocks) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.blockedBy) {
            target.blockedBy = [];
          }
          if (!target.blockedBy.includes(id)) {
            target.blockedBy.push(id);
            repairs.push(
              `Repaired bidirectional link: ${id} blocks ${targetId}`
            );
          }
        }
      }
    }

    // Repair blockedBy/blocks relationships
    if (workUnit.blockedBy) {
      for (const targetId of workUnit.blockedBy) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.blocks) {
            target.blocks = [];
          }
          if (!target.blocks.includes(id)) {
            target.blocks.push(id);
            repairs.push(
              `Repaired bidirectional link: ${targetId} blocks ${id}`
            );
          }
        }
      }
    }

    // Repair relatesTo relationships (bidirectional)
    if (workUnit.relatesTo) {
      for (const targetId of workUnit.relatesTo) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.relatesTo) {
            target.relatesTo = [];
          }
          if (!target.relatesTo.includes(id)) {
            target.relatesTo.push(id);
            repairs.push(
              `Repaired bidirectional link: ${id} relates to ${targetId}`
            );
          }
        }
      }
    }
  }

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async data => {
    Object.assign(data, workUnitsData);
  });

  return {
    success: true,
    repairs,
    repaired: repairs.length,
  };
}

export function registerRepairWorkUnitsCommand(program: Command): void {
  program
    .command('repair-work-units')
    .description('Repair work units data integrity issues')
    .option('--dry-run', 'Show what would be repaired without making changes')
    .action(async (options: { dryRun?: boolean }) => {
      try {
        const result = await repairWorkUnits({
          dryRun: options.dryRun,
        });
        console.log(chalk.green(`✓ Repaired ${result.repaired} issues`));
        if (result.details && result.details.length > 0) {
          result.details.forEach((detail: string) =>
            console.log(chalk.cyan(`  - ${detail}`))
          );
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to repair work units:'),
          error.message
        );
        process.exit(1);
      }
    });
}
