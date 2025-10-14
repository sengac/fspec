import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { addDependency } from './add-dependency';

interface AddDependenciesOptions {
  workUnitId: string;
  dependencies: {
    blocks?: string[];
    blockedBy?: string[];
    dependsOn?: string[];
    relatesTo?: string[];
  };
  cwd?: string;
}

interface AddDependenciesResult {
  success: boolean;
  added: number;
}

export async function addDependencies(
  options: AddDependenciesOptions
): Promise<AddDependenciesResult> {
  const cwd = options.cwd || process.cwd();
  let added = 0;

  // Add all blocks relationships
  if (options.dependencies.blocks) {
    for (const targetId of options.dependencies.blocks) {
      await addDependency({
        workUnitId: options.workUnitId,
        blocks: targetId,
        cwd,
      });
      added++;
    }
  }

  // Add all blockedBy relationships
  if (options.dependencies.blockedBy) {
    for (const targetId of options.dependencies.blockedBy) {
      await addDependency({
        workUnitId: options.workUnitId,
        blockedBy: targetId,
        cwd,
      });
      added++;
    }
  }

  // Add all dependsOn relationships
  if (options.dependencies.dependsOn) {
    for (const targetId of options.dependencies.dependsOn) {
      await addDependency({
        workUnitId: options.workUnitId,
        dependsOn: targetId,
        cwd,
      });
      added++;
    }
  }

  // Add all relatesTo relationships
  if (options.dependencies.relatesTo) {
    for (const targetId of options.dependencies.relatesTo) {
      await addDependency({
        workUnitId: options.workUnitId,
        relatesTo: targetId,
        cwd,
      });
      added++;
    }
  }

  return {
    success: true,
    added,
  };
}

export function registerAddDependenciesCommand(program: Command): void {
  program
    .command('add-dependencies')
    .description('Add multiple dependency relationships at once')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--blocks <ids...>', 'Work unit IDs that this blocks')
    .option('--blocked-by <ids...>', 'Work unit IDs that block this')
    .option('--depends-on <ids...>', 'Work unit IDs this depends on')
    .option('--relates-to <ids...>', 'Related work unit IDs')
    .action(
      async (
        workUnitId: string,
        options: {
          blocks?: string[];
          blockedBy?: string[];
          dependsOn?: string[];
          relatesTo?: string[];
        }
      ) => {
        try {
          const result = await addDependencies({
            workUnitId,
            dependencies: {
              blocks: options.blocks,
              blockedBy: options.blockedBy,
              dependsOn: options.dependsOn,
              relatesTo: options.relatesTo,
            },
          });
          console.log(
            chalk.green(`✓ Added ${result.added} dependencies successfully`)
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to add dependencies:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
