import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';
import {
  ensureWorkUnitsFile,
  ensurePrefixesFile,
  ensureEpicsFile,
} from '../utils/ensure-files';

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

export async function createWorkUnit(
  options: CreateWorkUnitOptions
): Promise<CreateWorkUnitResult> {
  const cwd = options.cwd || process.cwd();

  // Validate title
  if (!options.title || options.title.trim() === '') {
    throw new Error('Title is required');
  }

  // Read prefixes (auto-create if missing)
  const prefixesData = await ensurePrefixesFile(cwd);

  // Validate prefix is registered
  if (!prefixesData.prefixes[options.prefix]) {
    throw new Error(
      `Prefix '${options.prefix}' is not registered. Run 'fspec create-prefix ${options.prefix} "Description"' first.`
    );
  }

  // Read work units (auto-create if missing)
  const workUnitsData = await ensureWorkUnitsFile(cwd);

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
    const epicsData = await ensureEpicsFile(cwd);

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
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  // Update epic if provided
  if (options.epic) {
    const epicsFile = join(cwd, 'spec/epics.json');
    const epicsData = await ensureEpicsFile(cwd);

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

  const nextNumber =
    existingIds.length === 0 ? 1 : Math.max(...existingIds) + 1;
  return `${prefix}-${String(nextNumber).padStart(3, '0')}`;
}

function calculateNestingDepth(
  workUnitsData: WorkUnitsData,
  workUnitId: string,
  depth = 1
): number {
  const workUnit = workUnitsData.workUnits[workUnitId];
  if (!workUnit || !workUnit.parent) {
    return depth;
  }
  return calculateNestingDepth(workUnitsData, workUnit.parent, depth + 1);
}

// CLI wrapper function for Commander.js
export async function createWorkUnitCommand(
  prefix: string,
  title: string,
  options: { description?: string; epic?: string; parent?: string }
): Promise<void> {
  const chalk = await import('chalk').then(m => m.default);
  try {
    const result = await createWorkUnit({
      prefix,
      title,
      description: options.description,
      epic: options.epic,
      parent: options.parent,
    });

    if (result.success && result.workUnitId) {
      console.log(chalk.green(`✓ Created work unit ${result.workUnitId}`));
      console.log(chalk.gray(`  Title: ${title}`));
      if (options.description) {
        console.log(chalk.gray(`  Description: ${options.description}`));
      }
      if (options.epic) {
        console.log(chalk.gray(`  Epic: ${options.epic}`));
      }
      if (options.parent) {
        console.log(chalk.gray(`  Parent: ${options.parent}`));
      }
      process.exit(0);
    } else {
      console.error(chalk.red('✗ Failed to create work unit'));
      process.exit(1);
    }
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}

export function registerCreateWorkUnitCommand(program: Command): void {
  program
    .command('create-work-unit')
    .description('Create a new work unit')
    .argument('<prefix>', 'Work unit prefix (e.g., AUTH, DASH)')
    .argument('<title>', 'Work unit title')
    .option('-d, --description <description>', 'Work unit description')
    .option('-e, --epic <epic>', 'Epic ID to associate with')
    .option('-p, --parent <parent>', 'Parent work unit ID')
    .action(createWorkUnitCommand);
}
