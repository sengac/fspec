import { writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface AddDependencyOptions {
  workUnitId: string;
  blocks?: string;
  blockedBy?: string;
  dependsOn?: string;
  relatesTo?: string;
  cwd?: string;
}

interface AddDependencyResult {
  success: boolean;
}

function detectCircularDependency(
  workUnits: Record<string, any>,
  fromId: string,
  toId: string,
  visited: Set<string> = new Set(),
  path: string[] = []
): string | null {
  if (visited.has(toId)) {
    return null;
  }

  visited.add(toId);
  path.push(toId);

  // Check if we've reached back to the starting point
  if (toId === fromId && path.length > 1) {
    return path.join(' -> ');
  }

  // Follow the blocks relationships
  const workUnit = workUnits[toId];
  if (workUnit?.blocks) {
    for (const blockedId of workUnit.blocks) {
      const cycle = detectCircularDependency(workUnits, fromId, blockedId, new Set(visited), [...path]);
      if (cycle) {
        return cycle;
      }
    }
  }

  return null;
}

export async function addDependency(options: AddDependencyOptions): Promise<AddDependencyResult> {
  const cwd = options.cwd || process.cwd();

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Handle blocks relationship (bidirectional)
  if (options.blocks) {
    if (!data.workUnits[options.blocks]) {
      throw new Error(`Target work unit '${options.blocks}' does not exist`);
    }

    // Prevent self-dependency
    if (options.workUnitId === options.blocks) {
      throw new Error('Cannot create self-dependency');
    }

    // Check for duplicate
    if (workUnit.blocks?.includes(options.blocks)) {
      throw new Error('Dependency already exists');
    }

    // Check for circular dependency
    const cycle = detectCircularDependency(data.workUnits, options.workUnitId, options.blocks);
    if (cycle) {
      throw new Error(`Circular dependency detected: ${options.workUnitId} -> ${cycle}`);
    }

    // Add blocks relationship
    if (!workUnit.blocks) {
      workUnit.blocks = [];
    }
    workUnit.blocks.push(options.blocks);

    // Add reverse blockedBy relationship
    const targetWorkUnit = data.workUnits[options.blocks];
    if (!targetWorkUnit.blockedBy) {
      targetWorkUnit.blockedBy = [];
    }
    targetWorkUnit.blockedBy.push(options.workUnitId);

    // Auto-transition target to blocked state if not already blocked or done
    if (targetWorkUnit.status !== 'blocked' && targetWorkUnit.status !== 'done') {
      const oldStatus = targetWorkUnit.status;
      targetWorkUnit.status = 'blocked';

      // Update states arrays
      data.states[oldStatus] = data.states[oldStatus].filter(id => id !== options.blocks);
      if (!data.states.blocked) {
        data.states.blocked = [];
      }
      if (!data.states.blocked.includes(options.blocks)) {
        data.states.blocked.push(options.blocks);
      }
    }
  }

  // Handle blockedBy relationship (inverse of blocks, bidirectional)
  if (options.blockedBy) {
    if (!data.workUnits[options.blockedBy]) {
      throw new Error(`Target work unit '${options.blockedBy}' does not exist`);
    }

    if (options.workUnitId === options.blockedBy) {
      throw new Error('Cannot create self-dependency');
    }

    if (workUnit.blockedBy?.includes(options.blockedBy)) {
      throw new Error('Dependency already exists');
    }

    // Check for circular dependency (from blocker's perspective)
    const cycle = detectCircularDependency(data.workUnits, options.blockedBy, options.workUnitId);
    if (cycle) {
      throw new Error(`Circular dependency detected: ${options.blockedBy} -> ${cycle}`);
    }

    // Add blockedBy relationship
    if (!workUnit.blockedBy) {
      workUnit.blockedBy = [];
    }
    workUnit.blockedBy.push(options.blockedBy);

    // Add reverse blocks relationship
    const targetWorkUnit = data.workUnits[options.blockedBy];
    if (!targetWorkUnit.blocks) {
      targetWorkUnit.blocks = [];
    }
    targetWorkUnit.blocks.push(options.workUnitId);

    // Auto-transition this work unit to blocked state
    if (workUnit.status !== 'blocked' && workUnit.status !== 'done') {
      const oldStatus = workUnit.status;
      workUnit.status = 'blocked';

      data.states[oldStatus] = data.states[oldStatus].filter(id => id !== options.workUnitId);
      if (!data.states.blocked) {
        data.states.blocked = [];
      }
      if (!data.states.blocked.includes(options.workUnitId)) {
        data.states.blocked.push(options.workUnitId);
      }
    }
  }

  // Handle dependsOn relationship (unidirectional)
  if (options.dependsOn) {
    if (!data.workUnits[options.dependsOn]) {
      throw new Error(`Target work unit '${options.dependsOn}' does not exist`);
    }

    if (options.workUnitId === options.dependsOn) {
      throw new Error('Cannot create self-dependency');
    }

    if (workUnit.dependsOn?.includes(options.dependsOn)) {
      throw new Error('Dependency already exists');
    }

    if (!workUnit.dependsOn) {
      workUnit.dependsOn = [];
    }
    workUnit.dependsOn.push(options.dependsOn);
  }

  // Handle relatesTo relationship (bidirectional)
  if (options.relatesTo) {
    if (!data.workUnits[options.relatesTo]) {
      throw new Error(`Target work unit '${options.relatesTo}' does not exist`);
    }

    if (options.workUnitId === options.relatesTo) {
      throw new Error('Cannot create self-dependency');
    }

    if (workUnit.relatesTo?.includes(options.relatesTo)) {
      throw new Error('Dependency already exists');
    }

    if (!workUnit.relatesTo) {
      workUnit.relatesTo = [];
    }
    workUnit.relatesTo.push(options.relatesTo);

    // Add reverse relatesTo relationship
    const targetWorkUnit = data.workUnits[options.relatesTo];
    if (!targetWorkUnit.relatesTo) {
      targetWorkUnit.relatesTo = [];
    }
    if (!targetWorkUnit.relatesTo.includes(options.workUnitId)) {
      targetWorkUnit.relatesTo.push(options.workUnitId);
    }
  }

  workUnit.updatedAt = new Date().toISOString();

  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
  };
}
