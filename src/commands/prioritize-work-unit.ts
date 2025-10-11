import { writeFile } from 'fs/promises';
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

  // Validate work unit is in backlog
  if (workUnit.status !== 'backlog') {
    throw new Error(
      `Can only prioritize work units in backlog state. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate target work units exist if before/after specified
  if (options.before && !workUnitsData.workUnits[options.before]) {
    throw new Error(`Work unit '${options.before}' does not exist`);
  }
  if (options.after && !workUnitsData.workUnits[options.after]) {
    throw new Error(`Work unit '${options.after}' does not exist`);
  }

  const backlog = [...workUnitsData.states.backlog];

  // Remove work unit from current position
  const currentIndex = backlog.indexOf(options.workUnitId);
  if (currentIndex !== -1) {
    backlog.splice(currentIndex, 1);
  }

  // Determine new position
  let newIndex = 0;

  if (options.position === 'top') {
    newIndex = 0;
  } else if (options.position === 'bottom') {
    newIndex = backlog.length;
  } else if (typeof options.position === 'number') {
    newIndex = options.position;
  } else if (options.before) {
    newIndex = backlog.indexOf(options.before);
    if (newIndex === -1) newIndex = 0;
  } else if (options.after) {
    newIndex = backlog.indexOf(options.after) + 1;
    if (newIndex === 0) newIndex = backlog.length;
  }

  // Insert at new position
  backlog.splice(newIndex, 0, options.workUnitId);

  // Update states
  workUnitsData.states.backlog = backlog;

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  return { success: true };
}
