import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import { fileManager } from '../utils/file-manager';
import type { WorkUnitsData, EpicsData, WorkUnitType } from '../types';
import { ensureWorkUnitsFile, ensureEpicsFile } from '../utils/ensure-files';

interface UpdateWorkUnitOptions {
  workUnitId: string;
  title?: string;
  description?: string;
  epic?: string;
  parent?: string;
  type?: WorkUnitType;
  cwd?: string;
}

interface UpdateWorkUnitResult {
  success: boolean;
}

export async function updateWorkUnit(
  options: UpdateWorkUnitOptions
): Promise<UpdateWorkUnitResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  // Validate type immutability
  if (options.type !== undefined) {
    throw new Error(
      `Work unit type is immutable and cannot be changed after creation.\n\n` +
        `Current type: ${workUnitsData.workUnits[options.workUnitId].type || 'story'}\n` +
        `Attempted to change to: ${options.type}\n\n` +
        `If you need to change the type, Delete this work unit and create a new one with the correct type.`
    );
  }

  // Validate parent if provided (check for circular references)
  if (options.parent) {
    if (!workUnitsData.workUnits[options.parent]) {
      throw new Error(`Parent work unit '${options.parent}' does not exist`);
    }

    // Check for circular reference
    if (
      wouldCreateCircularReference(
        workUnitsData,
        options.workUnitId,
        options.parent
      )
    ) {
      throw new Error('Circular parent relationship detected');
    }
  }

  // Validate epic if provided
  if (options.epic !== undefined) {
    const epicsData: EpicsData = await ensureEpicsFile(cwd);

    if (!epicsData.epics[options.epic]) {
      throw new Error(`Epic '${options.epic}' does not exist`);
    }
  }

  // Update work unit fields
  if (options.title !== undefined) {
    workUnitsData.workUnits[options.workUnitId].title = options.title;
  }

  if (options.description !== undefined) {
    workUnitsData.workUnits[options.workUnitId].description =
      options.description;
  }

  if (options.epic !== undefined) {
    const oldEpic = workUnitsData.workUnits[options.workUnitId].epic;
    workUnitsData.workUnits[options.workUnitId].epic = options.epic;

    // Update epic references
    const epicsFile = join(cwd, 'spec/epics.json');

    // LOCK-002: Use fileManager.transaction() for atomic write
    await fileManager.transaction(epicsFile, async data => {
      // Remove from old epic if exists
      if (oldEpic && data.epics[oldEpic] && data.epics[oldEpic].workUnits) {
        data.epics[oldEpic].workUnits = data.epics[oldEpic].workUnits.filter(
          id => id !== options.workUnitId
        );
      }

      // Add to new epic
      if (!data.epics[options.epic!].workUnits) {
        data.epics[options.epic!].workUnits = [];
      }
      if (!data.epics[options.epic!].workUnits.includes(options.workUnitId)) {
        data.epics[options.epic!].workUnits.push(options.workUnitId);
      }
    });
  }

  if (options.parent !== undefined) {
    // Remove from old parent's children if exists
    const oldParent = workUnitsData.workUnits[options.workUnitId].parent;
    if (oldParent && workUnitsData.workUnits[oldParent]) {
      if (workUnitsData.workUnits[oldParent].children) {
        workUnitsData.workUnits[oldParent].children = workUnitsData.workUnits[
          oldParent
        ].children.filter(id => id !== options.workUnitId);
      }
    }

    // Set new parent
    workUnitsData.workUnits[options.workUnitId].parent = options.parent;

    // Add to new parent's children
    if (!workUnitsData.workUnits[options.parent].children) {
      workUnitsData.workUnits[options.parent].children = [];
    }
    if (
      !workUnitsData.workUnits[options.parent].children.includes(
        options.workUnitId
      )
    ) {
      workUnitsData.workUnits[options.parent].children.push(options.workUnitId);
    }
  }

  // Update timestamp
  workUnitsData.workUnits[options.workUnitId].updatedAt =
    new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async data => {
    Object.assign(data, workUnitsData);
  });

  return { success: true };
}

function wouldCreateCircularReference(
  workUnitsData: WorkUnitsData,
  workUnitId: string,
  proposedParentId: string,
  visited: Set<string> = new Set()
): boolean {
  // If we're trying to make a work unit its own ancestor, that's circular
  if (proposedParentId === workUnitId) {
    return true;
  }

  // If we've already visited this node, we have a cycle
  if (visited.has(proposedParentId)) {
    return true;
  }

  visited.add(proposedParentId);

  // Check if the proposed parent has the work unit as an ancestor
  const proposedParent = workUnitsData.workUnits[proposedParentId];
  if (!proposedParent) {
    return false;
  }

  // If the proposed parent has the workUnitId as a parent, that would create a circle
  if (proposedParent.parent) {
    return wouldCreateCircularReference(
      workUnitsData,
      workUnitId,
      proposedParent.parent,
      visited
    );
  }

  return false;
}

export function registerUpdateWorkUnitCommand(program: Command): void {
  program
    .command('update-work-unit')
    .description('Update work unit properties')
    .argument('<workUnitId>', 'Work unit ID to update')
    .option('-t, --title <title>', 'New title')
    .option('-d, --description <description>', 'New description')
    .option('-e, --epic <epic>', 'Epic ID')
    .option('-p, --parent <parent>', 'Parent work unit ID')
    .action(
      async (
        workUnitId: string,
        options: {
          title?: string;
          description?: string;
          epic?: string;
          parent?: string;
        }
      ) => {
        try {
          await updateWorkUnit({
            workUnitId,
            ...options,
          });
          console.log(
            chalk.green(`✓ Work unit ${workUnitId} updated successfully`)
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to update work unit:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
