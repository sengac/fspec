import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';

const WORK_UNIT_ID_REGEX = /^[A-Z]{2,6}-\d+$/;
const MAX_NESTING_DEPTH = 3;

interface CreateWorkUnitOptions {
  prefix: string;
  title: string;
  description?: string;
  epic?: string;
  parent?: string;
  cwd?: string;
}

interface CreateWorkUnitResult {
  success: boolean;
  workUnitId?: string;
}

export async function createWorkUnit(options: CreateWorkUnitOptions): Promise<CreateWorkUnitResult> {
  const cwd = options.cwd || process.cwd();

  // Validate title
  if (!options.title || options.title.trim() === '') {
    throw new Error('Title is required');
  }

  // Read prefixes
  const prefixesFile = join(cwd, 'spec/prefixes.json');
  const prefixesContent = await readFile(prefixesFile, 'utf-8');
  const prefixesData: PrefixesData = JSON.parse(prefixesContent);

  // Validate prefix is registered
  if (!prefixesData.prefixes[options.prefix]) {
    throw new Error(
      `Prefix '${options.prefix}' is not registered. Run 'fspec create-prefix ${options.prefix} "Description"' first.`
    );
  }

  // Read work units
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

  // Validate parent if provided
  if (options.parent) {
    if (!workUnitsData.workUnits[options.parent]) {
      throw new Error(`Parent work unit '${options.parent}' does not exist`);
    }

    // Check nesting depth
    const depth = calculateNestingDepth(workUnitsData, options.parent);
    if (depth >= MAX_NESTING_DEPTH) {
      throw new Error(`Maximum nesting depth (${MAX_NESTING_DEPTH}) exceeded`);
    }
  }

  // Validate epic if provided
  if (options.epic) {
    const epicsFile = join(cwd, 'spec/epics.json');
    const epicsContent = await readFile(epicsFile, 'utf-8');
    const epicsData: EpicsData = JSON.parse(epicsContent);

    if (!epicsData.epics[options.epic]) {
      throw new Error(`Epic '${options.epic}' does not exist`);
    }
  }

  // Generate next ID
  const nextId = generateNextId(workUnitsData, options.prefix);

  // Create work unit
  const now = new Date().toISOString();
  const newWorkUnit = {
    id: nextId,
    title: options.title,
    status: 'backlog' as const,
    createdAt: now,
    updatedAt: now,
    ...(options.description && { description: options.description }),
    ...(options.epic && { epic: options.epic }),
    ...(options.parent && { parent: options.parent }),
    ...(!options.parent && { children: [] }),
  };

  workUnitsData.workUnits[nextId] = newWorkUnit;

  // Add to states index
  if (!workUnitsData.states.backlog) {
    workUnitsData.states.backlog = [];
  }
  workUnitsData.states.backlog.push(nextId);

  // Update parent's children array if parent exists
  if (options.parent) {
    if (!workUnitsData.workUnits[options.parent].children) {
      workUnitsData.workUnits[options.parent].children = [];
    }
    workUnitsData.workUnits[options.parent].children.push(nextId);
  }

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  // Update epic if provided
  if (options.epic) {
    const epicsFile = join(cwd, 'spec/epics.json');
    const epicsContent = await readFile(epicsFile, 'utf-8');
    const epicsData: EpicsData = JSON.parse(epicsContent);

    if (!epicsData.epics[options.epic].workUnits) {
      epicsData.epics[options.epic].workUnits = [];
    }
    epicsData.epics[options.epic].workUnits.push(nextId);

    await writeFile(epicsFile, JSON.stringify(epicsData, null, 2));
  }

  return {
    success: true,
    workUnitId: nextId,
  };
}

function generateNextId(workUnitsData: WorkUnitsData, prefix: string): string {
  const existingIds = Object.keys(workUnitsData.workUnits)
    .filter(id => id.startsWith(`${prefix}-`))
    .map(id => parseInt(id.split('-')[1]))
    .filter(num => !isNaN(num));

  const nextNumber = existingIds.length === 0 ? 1 : Math.max(...existingIds) + 1;
  return `${prefix}-${String(nextNumber).padStart(3, '0')}`;
}

function calculateNestingDepth(workUnitsData: WorkUnitsData, workUnitId: string, depth = 1): number {
  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit || !workUnit.parent) {
    return depth;
  }
  return calculateNestingDepth(workUnitsData, workUnit.parent, depth + 1);
}
