import { writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface DeleteWorkUnitOptions {
  workUnitId: string;
  force?: boolean;
  skipConfirmation?: boolean;
  cascadeDependencies?: boolean;
  cwd?: string;
}

interface DeleteWorkUnitResult {
  success: boolean;
  warnings?: string[];
}

export async function deleteWorkUnit(options: DeleteWorkUnitOptions): Promise<DeleteWorkUnitResult> {
  const cwd = options.cwd || process.cwd();
  const warnings: string[] = [];

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];

  // Check if work unit has children
  if (workUnit.children && workUnit.children.length > 0) {
    throw new Error(
      `Cannot delete work unit with children: ${workUnit.children.join(', ')}. Delete children first or remove parent relationship.`
    );
  }

  // Check for dependencies
  const hasDependencies =
    (workUnit.blocks?.length || 0) > 0 ||
    (workUnit.blockedBy?.length || 0) > 0 ||
    (workUnit.dependsOn?.length || 0) > 0 ||
    (workUnit.relatesTo?.length || 0) > 0;

  if (hasDependencies && !options.cascadeDependencies) {
    throw new Error(
      `Work unit '${options.workUnitId}' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete.`
    );
  }

  // Warn if work unit blocks other work
  if (workUnit.blocks && workUnit.blocks.length > 0) {
    warnings.push(`This work unit blocks ${workUnit.blocks.length} work unit(s): ${workUnit.blocks.join(', ')}`);
  }

  // Clean up bidirectional dependencies if cascading
  if (options.cascadeDependencies) {
    // Remove blocks relationships
    if (workUnit.blocks) {
      for (const targetId of workUnit.blocks) {
        if (workUnitsData.workUnits[targetId]?.blockedBy) {
          workUnitsData.workUnits[targetId].blockedBy = workUnitsData.workUnits[targetId].blockedBy!.filter(
            id => id !== options.workUnitId
          );
          if (workUnitsData.workUnits[targetId].blockedBy!.length === 0) {
            delete workUnitsData.workUnits[targetId].blockedBy;
          }
        }
      }
    }

    // Remove blockedBy relationships
    if (workUnit.blockedBy) {
      for (const targetId of workUnit.blockedBy) {
        if (workUnitsData.workUnits[targetId]?.blocks) {
          workUnitsData.workUnits[targetId].blocks = workUnitsData.workUnits[targetId].blocks!.filter(
            id => id !== options.workUnitId
          );
          if (workUnitsData.workUnits[targetId].blocks!.length === 0) {
            delete workUnitsData.workUnits[targetId].blocks;
          }
        }
      }
    }

    // Remove relatesTo relationships (bidirectional)
    if (workUnit.relatesTo) {
      for (const targetId of workUnit.relatesTo) {
        if (workUnitsData.workUnits[targetId]?.relatesTo) {
          workUnitsData.workUnits[targetId].relatesTo = workUnitsData.workUnits[targetId].relatesTo!.filter(
            id => id !== options.workUnitId
          );
          if (workUnitsData.workUnits[targetId].relatesTo!.length === 0) {
            delete workUnitsData.workUnits[targetId].relatesTo;
          }
        }
      }
    }
  }

  // Remove from parent's children if it has a parent
  if (workUnit.parent && workUnitsData.workUnits[workUnit.parent]) {
    const parent = workUnitsData.workUnits[workUnit.parent];
    if (parent.children) {
      parent.children = parent.children.filter(id => id !== options.workUnitId);
    }
  }

  // Remove from states index
  for (const state of Object.keys(workUnitsData.states)) {
    if (workUnitsData.states[state]) {
      workUnitsData.states[state] = workUnitsData.states[state].filter(id => id !== options.workUnitId);
    }
  }

  // Delete work unit
  delete workUnitsData.workUnits[options.workUnitId];

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  return {
    success: true,
    ...(warnings.length > 0 && { warnings }),
  };
}
