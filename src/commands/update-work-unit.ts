import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, EpicsData, WorkItemType } from '../types';
import { ensureWorkUnitsFile, ensureEpicsFile } from '../utils/ensure-files';

interface UpdateWorkUnitOptions {
  workUnitId: string;
  title?: string;
  description?: string;
  epic?: string;
  parent?: string;
  type?: WorkItemType;
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
      `Work item type is immutable and cannot be changed after creation.\n\n` +
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
    const epicsData: EpicsData = await ensureEpicsFile(cwd);
    const epicsFile = join(cwd, 'spec/epics.json');

    // Remove from old epic if exists
    if (
      oldEpic &&
      epicsData.epics[oldEpic] &&
      epicsData.epics[oldEpic].workUnits
    ) {
      epicsData.epics[oldEpic].workUnits = epicsData.epics[
        oldEpic
      ].workUnits.filter(id => id !== options.workUnitId);
    }

    // Add to new epic
    if (!epicsData.epics[options.epic].workUnits) {
      epicsData.epics[options.epic].workUnits = [];
    }
    if (!epicsData.epics[options.epic].workUnits.includes(options.workUnitId)) {
      epicsData.epics[options.epic].workUnits.push(options.workUnitId);
    }

    await writeFile(epicsFile, JSON.stringify(epicsData, null, 2));
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

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

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
