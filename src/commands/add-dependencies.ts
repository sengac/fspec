import { readFile, writeFile } from 'fs/promises';
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

export async function addDependencies(options: AddDependenciesOptions): Promise<AddDependenciesResult> {
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
