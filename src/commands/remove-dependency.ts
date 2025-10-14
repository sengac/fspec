import { writeFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface RemoveDependencyOptions {
  workUnitId: string;
  blocks?: string;
  blockedBy?: string;
  dependsOn?: string;
  relatesTo?: string;
  cwd?: string;
}

interface RemoveDependencyResult {
  success: boolean;
}

export async function removeDependency(
  options: RemoveDependencyOptions
): Promise<RemoveDependencyResult> {
  const cwd = options.cwd || process.cwd();

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Remove blocks relationship (bidirectional cleanup)
  if (options.blocks) {
    if (workUnit.blocks) {
      workUnit.blocks = workUnit.blocks.filter(id => id !== options.blocks);
      if (workUnit.blocks.length === 0) {
        delete workUnit.blocks;
      }
    }

    // Remove reverse blockedBy relationship
    if (data.workUnits[options.blocks]) {
      const targetWorkUnit = data.workUnits[options.blocks];
      if (targetWorkUnit.blockedBy) {
        targetWorkUnit.blockedBy = targetWorkUnit.blockedBy.filter(
          id => id !== options.workUnitId
        );
        if (targetWorkUnit.blockedBy.length === 0) {
          delete targetWorkUnit.blockedBy;
        }
      }
    }
  }

  // Remove blockedBy relationship (bidirectional cleanup)
  if (options.blockedBy) {
    if (workUnit.blockedBy) {
      workUnit.blockedBy = workUnit.blockedBy.filter(
        id => id !== options.blockedBy
      );
      if (workUnit.blockedBy.length === 0) {
        delete workUnit.blockedBy;
      }
    }

    // Remove reverse blocks relationship
    if (data.workUnits[options.blockedBy]) {
      const targetWorkUnit = data.workUnits[options.blockedBy];
      if (targetWorkUnit.blocks) {
        targetWorkUnit.blocks = targetWorkUnit.blocks.filter(
          id => id !== options.workUnitId
        );
        if (targetWorkUnit.blocks.length === 0) {
          delete targetWorkUnit.blocks;
        }
      }
    }
  }

  // Remove dependsOn relationship (unidirectional)
  if (options.dependsOn) {
    if (workUnit.dependsOn) {
      workUnit.dependsOn = workUnit.dependsOn.filter(
        id => id !== options.dependsOn
      );
      if (workUnit.dependsOn.length === 0) {
        delete workUnit.dependsOn;
      }
    }
  }

  // Remove relatesTo relationship (bidirectional cleanup)
  if (options.relatesTo) {
    if (workUnit.relatesTo) {
      workUnit.relatesTo = workUnit.relatesTo.filter(
        id => id !== options.relatesTo
      );
      if (workUnit.relatesTo.length === 0) {
        delete workUnit.relatesTo;
      }
    }

    // Remove reverse relatesTo relationship
    if (data.workUnits[options.relatesTo]) {
      const targetWorkUnit = data.workUnits[options.relatesTo];
      if (targetWorkUnit.relatesTo) {
        targetWorkUnit.relatesTo = targetWorkUnit.relatesTo.filter(
          id => id !== options.workUnitId
        );
        if (targetWorkUnit.relatesTo.length === 0) {
          delete targetWorkUnit.relatesTo;
        }
      }
    }
  }

  workUnit.updatedAt = new Date().toISOString();

  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
  };
}

export function registerRemoveDependencyCommand(program: Command): void {
  program
    .command('remove-dependency')
    .description('Remove a dependency relationship between work units')
    .argument('<workUnitId>', 'Work unit ID')
    .argument(
      '[dependsOnId]',
      'Work unit ID to remove from dependsOn (shorthand for --depends-on)'
    )
    .option('--blocks <targetId>', 'Remove blocks relationship')
    .option('--blocked-by <targetId>', 'Remove blockedBy relationship')
    .option('--depends-on <targetId>', 'Remove dependsOn relationship')
    .option('--relates-to <targetId>', 'Remove relatesTo relationship')
    .action(
      async (
        workUnitId: string,
        dependsOnId: string | undefined,
        options: {
          blocks?: string;
          blockedBy?: string;
          dependsOn?: string;
          relatesTo?: string;
        }
      ) => {
        try {
          // If second argument provided, use it as --depends-on (shorthand syntax)
          const finalDependsOn = dependsOnId || options.dependsOn;

          // Check if user provided both shorthand and option (conflict)
          if (
            dependsOnId &&
            options.dependsOn &&
            dependsOnId !== options.dependsOn
          ) {
            throw new Error(
              'Cannot specify dependency both as argument and --depends-on option'
            );
          }

          // Require at least one relationship type
          if (
            !finalDependsOn &&
            !options.blocks &&
            !options.blockedBy &&
            !options.relatesTo
          ) {
            throw new Error(
              'Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to'
            );
          }

          await removeDependency({
            workUnitId,
            blocks: options.blocks,
            blockedBy: options.blockedBy,
            dependsOn: finalDependsOn,
            relatesTo: options.relatesTo,
          });
          console.log(chalk.green(`✓ Dependency removed successfully`));
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to remove dependency:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
