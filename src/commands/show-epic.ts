import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface Epic {
  id: string;
  title?: string;
  description?: string;
  [key: string]: unknown;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface WorkUnit {
  id: string;
  status?: string;
  epic?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface EpicProgress {
  epic: Epic;
  totalWorkUnits: number;
  completedWorkUnits: number;
  completionPercentage: number;
}

export async function showEpic(options: {
  epicId: string;
  cwd?: string;
}): Promise<EpicProgress> {
  const cwd = options.cwd || process.cwd();
  const epicsFile = join(cwd, 'spec', 'epics.json');
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Read epic
  let epicsData: EpicsData;
  try {
    const epicsContent = await readFile(epicsFile, 'utf-8');
    epicsData = JSON.parse(epicsContent);
  } catch (error: unknown) {
    // No epics file - epic doesn't exist
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      throw new Error(`Epic ${options.epicId} not found`);
    }
    throw error;
  }

  if (!epicsData.epics[options.epicId]) {
    throw new Error(`Epic ${options.epicId} not found`);
  }

  const epic = epicsData.epics[options.epicId];

  // Read work units to calculate progress
  let totalWorkUnits = 0;
  let completedWorkUnits = 0;

  try {
    const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
    const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

    // Count work units for this epic
    for (const workUnit of Object.values(workUnitsData.workUnits)) {
      if (workUnit.epic === options.epicId) {
        totalWorkUnits++;
        if (workUnit.status === 'done') {
          completedWorkUnits++;
        }
      }
    }
  } catch {
    // No work units file yet
  }

  const completionPercentage =
    totalWorkUnits > 0
      ? Math.round((completedWorkUnits / totalWorkUnits) * 100 * 100) / 100
      : 0;

  return {
    epic,
    totalWorkUnits,
    completedWorkUnits,
    completionPercentage,
  };
}

export async function showEpicCommand(
  epicId: string,
  options: { format?: string }
): Promise<void> {
  try {
    const result = await showEpic({
      epicId,
      output: (options.format as 'json' | 'text') || 'text',
    });

    if (options.format === 'json') {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(chalk.bold(`\nEpic: ${result.id}`));
      console.log('');
      console.log(chalk.cyan('Title:'), result.title);

      if (result.description) {
        console.log(chalk.cyan('Description:'), result.description);
      }

      if (result.workUnits && result.workUnits.length > 0) {
        console.log(chalk.cyan('\nWork Units:'));
        result.workUnits.forEach(wu => {
          console.log(`  - ${wu}`);
        });
      }

      console.log('');
      console.log(
        chalk.gray('Created:'),
        new Date(result.createdAt).toLocaleString()
      );
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
