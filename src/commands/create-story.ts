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

interface CreateStoryOptions {
  prefix: string;
  title: string;
  description?: string;
  epic?: string;
  parent?: string;
  cwd?: string;
}

interface CreateStoryResult {
  success: boolean;
  workUnitId?: string;
  systemReminder?: string;
}

export async function createStory(
  options: CreateStoryOptions
): Promise<CreateStoryResult> {
  const cwd = options.cwd || process.cwd();

  // Check if foundation.json exists
  const originalCommand = `fspec create-story ${options.prefix} "${options.title}"`;
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
      throw new Error(`Parent story '${options.parent}' does not exist`);
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

  // Create story
  const now = new Date().toISOString();
  const newStory = {
    id: nextId,
    title: options.title,
    type: 'story' as const,
    status: 'backlog' as const,
    createdAt: now,
    updatedAt: now,
    ...(options.description && { description: options.description }),
    ...(options.epic && { epic: options.epic }),
    ...(options.parent && { parent: options.parent }),
    ...(!options.parent && { children: [] }),
  };

  workUnitsData.workUnits[nextId] = newStory;

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

  // Generate Example Mapping system-reminder
  const systemReminder = `<system-reminder>
Story ${nextId} created successfully.

Next steps - Example Mapping:
  Stories use Example Mapping to define acceptance criteria through conversation.

  1. Set user story fields:
     fspec set-user-story ${nextId} --role "role" --action "action" --benefit "benefit"

  2. Add business rules (blue cards):
     fspec add-rule ${nextId} "Rule text"

  3. Add concrete examples (green cards):
     fspec add-example ${nextId} "Example text"

  4. Ask questions (red cards):
     fspec add-question ${nextId} "@human: Question text?"

  5. Generate scenarios from example map:
     fspec generate-scenarios ${nextId}

Example Mapping helps clarify requirements before writing code.

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
export async function createStoryCommand(
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
    const result = await createStory({
      prefix,
      title,
      description: options.description,
      epic: options.epic,
      parent: options.parent,
    });

    if (result.success && result.workUnitId) {
      console.log(chalk.green(`✓ Created story ${result.workUnitId}`));
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
      console.error(chalk.red('✗ Failed to create story'));
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

export function registerCreateStoryCommand(program: Command): void {
  program
    .command('create-story')
    .description('Create a new story with Example Mapping guidance')
    .argument('<prefix>', 'Story prefix (e.g., AUTH, DASH)')
    .argument('<title>', 'Story title')
    .option('-d, --description <description>', 'Story description')
    .option('-e, --epic <epic>', 'Epic ID to associate with')
    .option('-p, --parent <parent>', 'Parent story ID')
    .action(createStoryCommand);
}
