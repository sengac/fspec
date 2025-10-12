import { readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile, ensurePrefixesFile } from '../utils/ensure-files';

interface ListWorkUnitsOptions {
  status?: string;
  prefix?: string;
  epic?: string;
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
}): Promise<void> {
  const chalk = await import('chalk').then(m => m.default);
  try {
    const result = await listWorkUnits({
      status: options.status,
      prefix: options.prefix,
      epic: options.epic,
    });

    if (result.workUnits.length === 0) {
      console.log(chalk.yellow('No work units found'));
      process.exit(0);
    }

    console.log(chalk.bold(`\nWork Units (${result.workUnits.length})`));
    console.log('');

    for (const wu of result.workUnits) {
      console.log(chalk.cyan(wu.id) + chalk.gray(` [${wu.status}]`));
      console.log(`  ${wu.title}`);
      if (wu.epic) {
        console.log(chalk.gray(`  Epic: ${wu.epic}`));
      }
      console.log('');
    }

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
