import { writeFile } from 'fs/promises';
import { join } from 'path';
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
