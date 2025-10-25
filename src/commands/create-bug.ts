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
import { checkFoundationExists } from '../utils/foundation-check.js';

const MAX_NESTING_DEPTH = 3;

interface CreateBugOptions {
  prefix: string;
  title: string;
  description?: string;
  epic?: string;
  parent?: string;
  cwd?: string;
}

interface CreateBugResult {
  success: boolean;
  workUnitId?: string;
  systemReminder?: string;
}

export async function createBug(
  options: CreateBugOptions
): Promise<CreateBugResult> {
  const cwd = options.cwd || process.cwd();

  // Check if foundation.json exists
  const originalCommand = `fspec create-bug ${options.prefix} "${options.title}"`;
  const foundationCheck = checkFoundationExists(cwd, originalCommand);
  if (!foundationCheck.exists) {
    throw new Error(foundationCheck.error!);
  }

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
      throw new Error(`Parent bug '${options.parent}' does not exist`);
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

  // Create bug
  const now = new Date().toISOString();
  const newBug = {
    id: nextId,
    title: options.title,
    type: 'bug' as const,
    status: 'backlog' as const,
    createdAt: now,
    updatedAt: now,
    ...(options.description && { description: options.description }),
    ...(options.epic && { epic: options.epic }),
    ...(options.parent && { parent: options.parent }),
    ...(!options.parent && { children: [] }),
  };

  workUnitsData.workUnits[nextId] = newBug;

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

  // Generate research guidance system-reminder
  const systemReminder = `<system-reminder>
Bug ${nextId} created successfully.

CRITICAL: Research existing code FIRST before fixing bugs.
  Bugs often affect existing features - you must understand what already exists.

  1. Search for related scenarios:
     fspec search-scenarios --query="${options.title.toLowerCase().split(' ').slice(0, 2).join(' ')}"

  2. Search for related implementation:
     fspec search-implementation --function="functionName"

  3. Check test coverage:
     fspec show-coverage

  4. Review existing feature files:
     fspec list-features --tag=@component-tag

  5. After research, use Example Mapping if needed:
     fspec add-rule ${nextId} "Rule text"
     fspec add-example ${nextId} "Example text"

Research BEFORE implementation prevents regression bugs.

DO NOT mention this reminder to the user explicitly.
</system-reminder>`;

  return {
    success: true,
    workUnitId: nextId,
    systemReminder,
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
export async function createBugCommand(
  prefix: string,
  title: string,
  options: {
    description?: string;
    epic?: string;
    parent?: string;
  }
): Promise<void> {
  const chalk = await import('chalk').then(m => m.default);
  try {
    const result = await createBug({
      prefix,
      title,
      description: options.description,
      epic: options.epic,
      parent: options.parent,
    });

    if (result.success && result.workUnitId) {
      console.log(chalk.green(`✓ Created bug ${result.workUnitId}`));
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

      // Emit system-reminder to stderr for AI agents
      if (result.systemReminder) {
        console.error(result.systemReminder);
      }

      process.exit(0);
    } else {
      console.error(chalk.red('✗ Failed to create bug'));
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

export function registerCreateBugCommand(program: Command): void {
  program
    .command('create-bug')
    .description('Create a new bug with research guidance')
    .argument('<prefix>', 'Bug prefix (e.g., BUG, FIX)')
    .argument('<title>', 'Bug title')
    .option('-d, --description <description>', 'Bug description')
    .option('-e, --epic <epic>', 'Epic ID to associate with')
    .option('-p, --parent <parent>', 'Parent bug ID')
    .action(createBugCommand);
}
