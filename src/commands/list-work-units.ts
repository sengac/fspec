import { readFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, WorkUnitType } from '../types';
import { ensureWorkUnitsFile, ensurePrefixesFile } from '../utils/ensure-files';

import { output } from '../utils/output';
interface ListWorkUnitsOptions {
  status?: string;
  prefix?: string;
  epic?: string;
  type?: WorkUnitType;
  cwd?: string;
}

interface WorkUnitSummary {
  id: string;
  title: string;
  status: string;
  epic?: string;
  [key: string]: unknown;
}

interface ListWorkUnitsResult {
  workUnits: WorkUnitSummary[];
}

export async function listWorkUnits(
  options: ListWorkUnitsOptions = {}
): Promise<ListWorkUnitsResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units (auto-create if missing)
  const workUnitsData = await ensureWorkUnitsFile(cwd);

  // Ensure prefixes file exists too (for consistency)
  await ensurePrefixesFile(cwd);

  // Get all work units
  let workUnits = Object.values(workUnitsData.workUnits);

  // Apply filters
  if (options.status) {
    workUnits = workUnits.filter(wu => wu.status === options.status);
  }

  if (options.prefix) {
    workUnits = workUnits.filter(wu => wu.id.startsWith(`${options.prefix}-`));
  }

  if (options.epic) {
    workUnits = workUnits.filter(wu => wu.epic === options.epic);
  }

  if (options.type) {
    workUnits = workUnits.filter(wu => {
      const type = wu.type || 'story'; // Default to 'story' for backward compatibility
      return type === options.type;
    });
  }

  // Map to summary format
  const summaries: WorkUnitSummary[] = workUnits.map(wu => ({
    id: wu.id,
    title: wu.title,
    status: wu.status,
    ...(wu.epic && { epic: wu.epic }),
  }));

  return {
    workUnits: summaries,
  };
}

// CLI wrapper function for Commander.js
export async function listWorkUnitsCommand(options: {
  status?: string;
  prefix?: string;
  epic?: string;
  type?: WorkUnitType;
  format?: string;
}): Promise<void> {
  const chalk = await import('chalk').then(m => m.default);
  try {
    const result = await listWorkUnits({
      status: options.status,
      prefix: options.prefix,
      epic: options.epic,
      type: options.type,
    });

    // JSON output for programmatic access
    if (options.format === 'json') {
      output.log(JSON.stringify(result, null, 2));
      return;
    }

    if (result.workUnits.length === 0) {
      output.log(chalk.yellow('No work units found'));
      process.exit(0);
    }

    output.log(chalk.bold(`\nWork Units (${result.workUnits.length})`));
    output.log('');

    for (const wu of result.workUnits) {
      output.log(chalk.cyan(wu.id) + chalk.gray(` [${wu.status}]`));
      output.log(`  ${wu.title}`);
      if (wu.epic) {
        output.log(chalk.gray(`  Epic: ${wu.epic}`));
      }
      output.log('');
    }

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      output.error(chalk.red('Error:'), error.message);
    } else {
      output.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}

export function registerListWorkUnitsCommand(program: Command): void {
  program
    .command('list-work-units')
    .description('List all work units')
    .option('-s, --status <status>', 'Filter by status')
    .option('-p, --prefix <prefix>', 'Filter by prefix')
    .option('-e, --epic <epic>', 'Filter by epic')
    .option(
      '-t, --type <type>',
      'Filter by work unit type: story, task, or bug'
    )
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (options: any) => {
      await listWorkUnitsCommand(options);
    });
}
